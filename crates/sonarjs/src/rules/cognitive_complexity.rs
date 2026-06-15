//! Rule `cognitive-complexity` (SonarJS key S3776).
//!
//! Clean-room port. Reports any function whose cognitive complexity exceeds the
//! configured threshold. Cognitive complexity differs from cyclomatic complexity
//! in two key ways: (1) nesting adds a penalty on top of the base increment,
//! and (2) structural constructs are counted individually rather than by their
//! decision points.
//!
//! **Algorithm summary** (computed per function body, with nested functions
//! accruing to the same outer total at increased nesting depth):
//!
//! - *Structural elements* (`if`, `?:`, `switch`, `for`, `for-in`, `for-of`,
//!   `while`, `do-while`, `catch`): add `1 + nesting`, then traverse body
//!   with `nesting + 1`. `else` and `else if` each add a flat `+1` only.
//! - *Logical operator sequences* (`&&`, `||`): add `+1` per maximal run of
//!   the same operator (operator-change sequences). `??` is not counted.
//! - *Labelled jumps* (`break <label>`, `continue <label>`): add flat `+1`.
//! - *Nested functions/arrows*: traverse their body at `nesting + 1`; their
//!   increments contribute to the **same** running total.
//!
//! A diagnostic fires when the total is **strictly greater than** the configured
//! `threshold` option (default **15**).
//!
//! Behaviour is derived from the public RSPEC S3776 description and the
//! canonical test matrix only; no upstream source, tests, fixtures, or message
//! strings were consulted or copied.

use oxc_ast::ast::{
    ArrowFunctionExpression, Expression, Function, IfStatement, Statement, SwitchStatement,
    TryStatement,
};
use oxc_syntax::operator::LogicalOperator;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "cognitive-complexity";

// ---------------------------------------------------------------------------
// Recursive body scorer (standalone, no scanner state)
// ---------------------------------------------------------------------------

/// Score the cognitive complexity of a slice of statements at the given nesting
/// depth. Returns the total score to be added to the enclosing function's total.
fn score_stmts(stmts: &[Statement<'_>], nesting: u32) -> u32 {
    let mut total: u32 = 0;
    for stmt in stmts {
        total = total.saturating_add(score_stmt(stmt, nesting));
    }
    total
}

fn score_stmt(stmt: &Statement<'_>, nesting: u32) -> u32 {
    match stmt {
        Statement::BlockStatement(block) => score_stmts(&block.body, nesting),

        Statement::IfStatement(if_stmt) => score_if(if_stmt, nesting),

        Statement::SwitchStatement(sw) => score_switch(sw, nesting),

        Statement::ForStatement(f) => {
            let test_score = f.test.as_ref().map_or(0, |e| score_expr(e, nesting));
            let init_score = f.init.as_ref().map_or(0, |i| {
                if let Some(e) = i.as_expression() {
                    score_expr(e, nesting)
                } else {
                    0
                }
            });
            let update_score = f.update.as_ref().map_or(0, |e| score_expr(e, nesting));
            (1u32 + nesting)
                .saturating_add(init_score)
                .saturating_add(test_score)
                .saturating_add(update_score)
                .saturating_add(score_stmt(&f.body, nesting + 1))
        }

        Statement::ForInStatement(f) => {
            (1u32 + nesting).saturating_add(score_stmt(&f.body, nesting + 1))
        }

        Statement::ForOfStatement(f) => {
            (1u32 + nesting).saturating_add(score_stmt(&f.body, nesting + 1))
        }

        Statement::WhileStatement(w) => {
            let test_score = score_expr(&w.test, nesting);
            (1u32 + nesting)
                .saturating_add(test_score)
                .saturating_add(score_stmt(&w.body, nesting + 1))
        }

        Statement::DoWhileStatement(d) => {
            let test_score = score_expr(&d.test, nesting);
            (1u32 + nesting)
                .saturating_add(test_score)
                .saturating_add(score_stmt(&d.body, nesting + 1))
        }

        Statement::TryStatement(t) => score_try(t, nesting),

        Statement::BreakStatement(b) => u32::from(b.label.is_some()),

        Statement::ContinueStatement(c) => u32::from(c.label.is_some()),

        Statement::ExpressionStatement(e) => score_expr(&e.expression, nesting),

        Statement::ReturnStatement(r) => r.argument.as_ref().map_or(0, |e| score_expr(e, nesting)),

        Statement::ThrowStatement(t) => score_expr(&t.argument, nesting),

        Statement::LabeledStatement(l) => score_stmt(&l.body, nesting),

        Statement::FunctionDeclaration(f) => score_function_body(f, nesting + 1),

        Statement::VariableDeclaration(v) => v
            .declarations
            .iter()
            .map(|d| d.init.as_ref().map_or(0, |e| score_expr(e, nesting)))
            .fold(0u32, |a, b| a.saturating_add(b)),

        _ => 0,
    }
}

fn score_if(if_stmt: &IfStatement<'_>, nesting: u32) -> u32 {
    // +1 + nesting for the `if` itself
    let mut score = 1u32 + nesting;
    // Score test expression (logical operators in conditions)
    score = score.saturating_add(score_expr_logical_only(&if_stmt.test, nesting));
    // Score the consequent body at nesting+1
    score = score.saturating_add(score_stmt(&if_stmt.consequent, nesting + 1));
    // Handle else / else-if
    if let Some(alternate) = &if_stmt.alternate {
        // +1 flat for else/else-if
        score = score.saturating_add(1);
        match alternate {
            Statement::IfStatement(else_if) => {
                // else-if: do NOT add another 1+nesting; just score the tail
                score = score.saturating_add(score_if_tail(else_if, nesting));
            }
            _ => {
                // else block: score at nesting+1
                score = score.saturating_add(score_stmt(alternate, nesting + 1));
            }
        }
    }
    score
}

/// Score the tail of an else-if chain. The `1` for the `else if` keyword has
/// already been counted by the parent; here we only score the consequent body
/// and any further alternate.
fn score_if_tail(if_stmt: &IfStatement<'_>, nesting: u32) -> u32 {
    // Score test at the same nesting (the "else if" +1 was counted by caller)
    let mut score = score_expr_logical_only(&if_stmt.test, nesting);
    // Body at nesting+1 (same as the parent if's nesting+1)
    score = score.saturating_add(score_stmt(&if_stmt.consequent, nesting + 1));
    if let Some(alternate) = &if_stmt.alternate {
        // +1 flat for the else/else-if
        score = score.saturating_add(1);
        match alternate {
            Statement::IfStatement(else_if) => {
                score = score.saturating_add(score_if_tail(else_if, nesting));
            }
            _ => {
                score = score.saturating_add(score_stmt(alternate, nesting + 1));
            }
        }
    }
    score
}

fn score_switch(sw: &SwitchStatement<'_>, nesting: u32) -> u32 {
    let mut score = 1u32 + nesting;
    for case in &sw.cases {
        score = score.saturating_add(score_stmts(&case.consequent, nesting + 1));
    }
    score
}

fn score_try(t: &TryStatement<'_>, nesting: u32) -> u32 {
    let mut score = score_stmts(&t.block.body, nesting);
    if let Some(catch) = &t.handler {
        // catch clause: +1 + nesting, body at nesting+1
        score = score.saturating_add(1u32 + nesting);
        score = score.saturating_add(score_stmts(&catch.body.body, nesting + 1));
    }
    if let Some(finally) = &t.finalizer {
        score = score.saturating_add(score_stmts(&finally.body, nesting));
    }
    score
}

/// Score a `Function` node whose body contributes to the outer function at
/// the given nesting depth.
fn score_function_body(f: &Function<'_>, nesting: u32) -> u32 {
    f.body
        .as_ref()
        .map_or(0, |b| score_stmts(&b.statements, nesting))
}

fn score_arrow_body(f: &ArrowFunctionExpression<'_>, nesting: u32) -> u32 {
    if f.expression {
        // Arrow with expression body: the first statement is an ExpressionStatement
        f.body
            .statements
            .first()
            .map_or(0, |s| score_stmt(s, nesting))
    } else {
        score_stmts(&f.body.statements, nesting)
    }
}

/// Score an expression for ALL contributions: logical chains, conditional
/// expressions, nested functions, and everything that may contain them.
fn score_expr(expr: &Expression<'_>, nesting: u32) -> u32 {
    match expr {
        Expression::LogicalExpression(logical) => score_logical_chain(logical, None, nesting),

        Expression::ConditionalExpression(cond) => {
            // +1 + nesting for the ternary
            let mut score = 1u32 + nesting;
            // test may have logical operators
            score = score.saturating_add(score_expr_logical_only(&cond.test, nesting));
            score = score.saturating_add(score_expr(&cond.consequent, nesting + 1));
            score = score.saturating_add(score_expr(&cond.alternate, nesting + 1));
            score
        }

        Expression::FunctionExpression(f) => score_function_body(f, nesting + 1),

        Expression::ArrowFunctionExpression(f) => score_arrow_body(f, nesting + 1),

        Expression::AssignmentExpression(e) => score_expr(&e.right, nesting),

        Expression::SequenceExpression(seq) => seq
            .expressions
            .iter()
            .map(|e| score_expr(e, nesting))
            .fold(0u32, |a, b| a.saturating_add(b)),

        Expression::CallExpression(call) => {
            let mut score = score_expr(&call.callee, nesting);
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    score = score.saturating_add(score_expr(e, nesting));
                }
            }
            score
        }

        Expression::NewExpression(call) => {
            let mut score = score_expr(&call.callee, nesting);
            for arg in &call.arguments {
                if let Some(e) = arg.as_expression() {
                    score = score.saturating_add(score_expr(e, nesting));
                }
            }
            score
        }

        Expression::BinaryExpression(bin) => {
            score_expr(&bin.left, nesting).saturating_add(score_expr(&bin.right, nesting))
        }

        Expression::UnaryExpression(u) => score_expr(&u.argument, nesting),

        Expression::StaticMemberExpression(m) => score_expr(&m.object, nesting),

        Expression::ComputedMemberExpression(m) => {
            score_expr(&m.object, nesting).saturating_add(score_expr(&m.expression, nesting))
        }

        Expression::PrivateFieldExpression(m) => score_expr(&m.object, nesting),

        Expression::TemplateLiteral(t) => t
            .expressions
            .iter()
            .map(|e| score_expr(e, nesting))
            .fold(0u32, |a, b| a.saturating_add(b)),

        Expression::TaggedTemplateExpression(t) => {
            let tag_score = score_expr(&t.tag, nesting);
            let quasi_score = t
                .quasi
                .expressions
                .iter()
                .map(|e| score_expr(e, nesting))
                .fold(0u32, |a, b| a.saturating_add(b));
            tag_score.saturating_add(quasi_score)
        }

        Expression::YieldExpression(y) => y.argument.as_ref().map_or(0, |e| score_expr(e, nesting)),

        Expression::AwaitExpression(a) => score_expr(&a.argument, nesting),

        Expression::ParenthesizedExpression(p) => score_expr(&p.expression, nesting),

        // Literals, identifiers, `this`, `super`, `import.meta`, etc. — no complexity
        _ => 0,
    }
}

/// Score only the logical operator contributions of an expression (for use in
/// `if` test conditions where the `if` itself is counted separately). Delegates
/// to `score_expr` which handles everything including logical chains.
fn score_expr_logical_only(expr: &Expression<'_>, nesting: u32) -> u32 {
    // We want to score logical chains and nested structures (functions/ternary)
    // that appear inside conditions. score_expr already does exactly this.
    score_expr(expr, nesting)
}

/// Score a logical expression chain. `parent_op` is the operator of the
/// containing `LogicalExpression` (if any), used to detect same-operator
/// chains. Adds `+1` for each maximal run of the same operator.
fn score_logical_chain(
    logical: &oxc_ast::ast::LogicalExpression<'_>,
    parent_op: Option<LogicalOperator>,
    nesting: u32,
) -> u32 {
    // Only && and || contribute; ?? is intentionally ignored
    let is_counted = matches!(logical.operator, LogicalOperator::And | LogicalOperator::Or);

    let mut score: u32 = 0;

    // Add +1 if this starts a new run (different operator from parent, or root)
    if is_counted && parent_op != Some(logical.operator) {
        score = 1;
    }

    let child_parent = if is_counted {
        Some(logical.operator)
    } else {
        None
    };

    // Score left sub-expression
    match &logical.left {
        Expression::LogicalExpression(left_logical) => {
            score = score.saturating_add(score_logical_chain(left_logical, child_parent, nesting));
        }
        other => {
            score = score.saturating_add(score_expr(other, nesting));
        }
    }

    // Score right sub-expression
    match &logical.right {
        Expression::LogicalExpression(right_logical) => {
            score = score.saturating_add(score_logical_chain(right_logical, child_parent, nesting));
        }
        other => {
            score = score.saturating_add(score_expr(other, nesting));
        }
    }

    score
}

// ---------------------------------------------------------------------------
// Scanner integration
// ---------------------------------------------------------------------------

impl Scanner<'_> {
    /// Score the body of a regular `Function` node and report if needed.
    /// Only runs when this is the outermost function (not already inside one
    /// whose body is being scored recursively).
    pub(crate) fn check_cognitive_complexity_fn(&mut self, f: &Function<'_>) {
        if !self.options.has_rule(RULE_NAME) || self.cognitive_complexity_fn_depth > 0 {
            return;
        }
        let body_score = score_function_body(f, 0);
        if body_score > self.options.cognitive_complexity_threshold {
            self.report(RULE_NAME, "cognitiveComplexity", f.span);
        }
    }

    /// Score the body of an `ArrowFunctionExpression` and report if needed.
    /// Only runs when this is the outermost function.
    pub(crate) fn check_cognitive_complexity_arrow(&mut self, f: &ArrowFunctionExpression<'_>) {
        if !self.options.has_rule(RULE_NAME) || self.cognitive_complexity_fn_depth > 0 {
            return;
        }
        let body_score = score_arrow_body(f, 0);
        if body_score > self.options.cognitive_complexity_threshold {
            self.report(RULE_NAME, "cognitiveComplexity", f.span);
        }
    }
}
