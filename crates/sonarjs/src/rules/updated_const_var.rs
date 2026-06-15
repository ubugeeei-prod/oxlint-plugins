//! Rule `updated-const-var` (SonarJS key S3500).
//!
//! Clean-room port from observed behaviour. A `const` binding cannot be
//! reassigned; this rule reports writes that target the binding itself:
//!
//! - assignment expressions (`x = 1`, `x += 1`)
//! - update expressions (`x++`, `--x`)
//! - destructuring assignment targets (`({ x } = obj)`, `[x] = arr`)
//! - for-in/for-of assignment targets (`for (x in obj)`, `for (x of xs)`)
//!
//! Property writes through a const object (`obj.x = 1`) are intentionally not
//! reported because they do not reassign the `obj` binding. Resolution uses Oxc
//! semantic symbols, so shadowed names are handled by identity rather than by
//! source text.

use oxc_ast::ast::{
    AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty, Expression,
    ForStatementLeft, IdentifierReference, SimpleAssignmentTarget,
};
use oxc_semantic::SymbolId;
use oxc_syntax::symbol::SymbolFlags;
use oxlint_plugins_carton::CompactString;

use crate::{DiagnosticData, scanner::Scanner};

pub(crate) const RULE_NAME: &str = "updated-const-var";

impl<'a> Scanner<'a> {
    fn updated_const_symbol_id(&self, ident: &IdentifierReference<'a>) -> Option<SymbolId> {
        let scoping = self.scoping?;
        let reference_id = ident.reference_id.get()?;
        let symbol_id = scoping.get_reference(reference_id).symbol_id()?;
        scoping
            .symbol_flags(symbol_id)
            .contains(SymbolFlags::ConstVariable)
            .then_some(symbol_id)
    }

    fn report_updated_const_var(&mut self, ident: &IdentifierReference<'a>) {
        if self.updated_const_symbol_id(ident).is_none() {
            return;
        }
        let data = DiagnosticData {
            value: Some(CompactString::from(ident.name.as_str())),
        };
        self.report_with_data(RULE_NAME, "updateConst", data, ident.span, None);
    }

    fn check_updated_const_var_expression_target(&mut self, expr: &Expression<'a>) {
        let Expression::Identifier(ident) = expr.get_inner_expression() else {
            return;
        };
        self.report_updated_const_var(ident);
    }

    fn check_updated_const_var_target(&mut self, target: &AssignmentTarget<'a>) {
        match target {
            AssignmentTarget::AssignmentTargetIdentifier(ident) => {
                self.report_updated_const_var(ident);
            }
            AssignmentTarget::TSAsExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTarget::TSSatisfiesExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTarget::TSNonNullExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTarget::TSTypeAssertion(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTarget::ArrayAssignmentTarget(target) => {
                for element in target.elements.iter().flatten() {
                    self.check_updated_const_var_maybe_default(element);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(target) => {
                for property in &target.properties {
                    self.check_updated_const_var_property(property);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            AssignmentTarget::ComputedMemberExpression(_)
            | AssignmentTarget::StaticMemberExpression(_)
            | AssignmentTarget::PrivateFieldExpression(_) => {}
        }
    }

    fn check_updated_const_var_maybe_default(&mut self, target: &AssignmentTargetMaybeDefault<'a>) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(target) => {
                self.check_updated_const_var_target(&target.binding);
            }
            AssignmentTargetMaybeDefault::AssignmentTargetIdentifier(ident) => {
                self.report_updated_const_var(ident);
            }
            AssignmentTargetMaybeDefault::TSAsExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTargetMaybeDefault::TSSatisfiesExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTargetMaybeDefault::TSNonNullExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTargetMaybeDefault::TSTypeAssertion(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            AssignmentTargetMaybeDefault::ArrayAssignmentTarget(target) => {
                for element in target.elements.iter().flatten() {
                    self.check_updated_const_var_maybe_default(element);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            AssignmentTargetMaybeDefault::ObjectAssignmentTarget(target) => {
                for property in &target.properties {
                    self.check_updated_const_var_property(property);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            AssignmentTargetMaybeDefault::ComputedMemberExpression(_)
            | AssignmentTargetMaybeDefault::StaticMemberExpression(_)
            | AssignmentTargetMaybeDefault::PrivateFieldExpression(_) => {}
        }
    }

    fn check_updated_const_var_property(&mut self, property: &AssignmentTargetProperty<'a>) {
        match property {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => {
                self.report_updated_const_var(&property.binding);
            }
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                self.check_updated_const_var_maybe_default(&property.binding);
            }
        }
    }

    fn check_updated_const_var_simple_target(&mut self, target: &SimpleAssignmentTarget<'a>) {
        match target {
            SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
                self.report_updated_const_var(ident);
            }
            SimpleAssignmentTarget::TSAsExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            SimpleAssignmentTarget::TSSatisfiesExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            SimpleAssignmentTarget::TSNonNullExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            SimpleAssignmentTarget::TSTypeAssertion(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            SimpleAssignmentTarget::ComputedMemberExpression(_)
            | SimpleAssignmentTarget::StaticMemberExpression(_)
            | SimpleAssignmentTarget::PrivateFieldExpression(_) => {}
        }
    }

    pub(crate) fn check_updated_const_var_assignment(&mut self, target: &AssignmentTarget<'a>) {
        self.check_updated_const_var_target(target);
    }

    pub(crate) fn check_updated_const_var_update(&mut self, target: &SimpleAssignmentTarget<'a>) {
        self.check_updated_const_var_simple_target(target);
    }

    pub(crate) fn check_updated_const_var_for_left(&mut self, left: &ForStatementLeft<'a>) {
        match left {
            ForStatementLeft::VariableDeclaration(_) => {}
            ForStatementLeft::AssignmentTargetIdentifier(ident) => {
                self.report_updated_const_var(ident);
            }
            ForStatementLeft::TSAsExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            ForStatementLeft::TSSatisfiesExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            ForStatementLeft::TSNonNullExpression(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            ForStatementLeft::TSTypeAssertion(expr) => {
                self.check_updated_const_var_expression_target(&expr.expression);
            }
            ForStatementLeft::ArrayAssignmentTarget(target) => {
                for element in target.elements.iter().flatten() {
                    self.check_updated_const_var_maybe_default(element);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            ForStatementLeft::ObjectAssignmentTarget(target) => {
                for property in &target.properties {
                    self.check_updated_const_var_property(property);
                }
                if let Some(rest) = &target.rest {
                    self.check_updated_const_var_target(&rest.target);
                }
            }
            ForStatementLeft::ComputedMemberExpression(_)
            | ForStatementLeft::StaticMemberExpression(_)
            | ForStatementLeft::PrivateFieldExpression(_) => {}
        }
    }
}
