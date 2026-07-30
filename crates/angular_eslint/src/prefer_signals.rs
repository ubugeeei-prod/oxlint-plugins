use oxc_ast::ast::{
    Decorator, Expression, Program, PropertyDefinition, Statement, TSType, TSTypeName,
};
use oxc_span::{GetSpan, Span};
use oxlint_plugins_carton::{CompactString, SmallVec};
use serde_json::Value;

use crate::scanner::Scanner;
use crate::types::{Diagnostic, DiagnosticDatum};

const PREFER_SIGNALS: &str = "prefer-signals";

const KNOWN_SIGNAL_TYPES: [&str; 5] = [
    "InputSignal",
    "ModelSignal",
    "Signal",
    "WritableSignal",
    "InputSignalWithTransform",
];

const KNOWN_SIGNAL_CREATION_FUNCTIONS: [&str; 10] = [
    "computed",
    "contentChild",
    "contentChildren",
    "input",
    "linkedSignal",
    "model",
    "signal",
    "toSignal",
    "viewChild",
    "viewChildren",
];

#[derive(Debug)]
struct PreferSignalsOptions<'a> {
    prefer_readonly_signal_properties: bool,
    prefer_input_signals: bool,
    prefer_query_signals: bool,
    use_type_checking: bool,
    additional_signal_creation_functions: SmallVec<[&'a str; 4]>,
}

impl Default for PreferSignalsOptions<'_> {
    fn default() -> Self {
        Self {
            prefer_readonly_signal_properties: true,
            prefer_input_signals: true,
            prefer_query_signals: true,
            use_type_checking: false,
            additional_signal_creation_functions: SmallVec::new(),
        }
    }
}

impl Scanner<'_> {
    pub(crate) fn collect_signal_returning_functions(&mut self, program: &Program<'_>) {
        if !self.options.is_enabled(PREFER_SIGNALS)
            || !configured_options(self.options).use_type_checking
        {
            return;
        }

        for statement in &program.body {
            let Statement::FunctionDeclaration(function) = statement else {
                continue;
            };
            let (Some(identifier), Some(return_type)) = (&function.id, &function.return_type)
            else {
                continue;
            };
            if type_annotation_is_known_signal(&return_type.type_annotation) {
                self.signal_returning_functions
                    .push(CompactString::from(identifier.name.as_str()));
            }
        }
    }

    pub(crate) fn check_prefer_signals_property(&mut self, property: &PropertyDefinition<'_>) {
        if !self.options.is_enabled(PREFER_SIGNALS) || property.readonly {
            return;
        }
        let options = configured_options(self.options);
        if !options.prefer_readonly_signal_properties
            || !property_is_signal(property, &options, &self.signal_returning_functions)
        {
            return;
        }
        self.report_prefer_signals(
            "preferReadonlySignalProperties",
            SmallVec::new(),
            property.key.span(),
        );
    }

    pub(crate) fn check_prefer_signals_decorator(&mut self, decorator: &Decorator<'_>) {
        if !self.options.is_enabled(PREFER_SIGNALS) {
            return;
        }
        let options = configured_options(self.options);
        let Expression::CallExpression(call) = &decorator.expression else {
            return;
        };
        let Expression::Identifier(identifier) = &call.callee else {
            return;
        };

        match identifier.name.as_str() {
            "Input" if options.prefer_input_signals => {
                self.report_prefer_signals("preferInputSignals", SmallVec::new(), decorator.span);
            }
            decorator_name
            @ ("ContentChild" | "ContentChildren" | "ViewChild" | "ViewChildren")
                if options.prefer_query_signals =>
            {
                let function_name = match decorator_name {
                    "ContentChild" => "contentChild",
                    "ContentChildren" => "contentChildren",
                    "ViewChild" => "viewChild",
                    "ViewChildren" => "viewChildren",
                    _ => unreachable!("guarded query decorator"),
                };
                let mut data = SmallVec::new();
                data.push(DiagnosticDatum {
                    key: CompactString::from("function"),
                    value: CompactString::from(function_name),
                });
                data.push(DiagnosticDatum {
                    key: CompactString::from("decorator"),
                    value: CompactString::from(decorator_name),
                });
                self.report_prefer_signals("preferQuerySignals", data, decorator.span);
            }
            _ => {}
        }
    }

    fn report_prefer_signals(
        &mut self,
        message_id: &'static str,
        data: SmallVec<[DiagnosticDatum; 2]>,
        span: Span,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name: PREFER_SIGNALS,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

fn configured_options(scan_options: &crate::ScanOptions) -> PreferSignalsOptions<'_> {
    let Some(option) = scan_options
        .options
        .as_array()
        .and_then(|options| options.first())
    else {
        return PreferSignalsOptions::default();
    };

    PreferSignalsOptions {
        prefer_readonly_signal_properties: boolean_option(
            option,
            "preferReadonlySignalProperties",
            true,
        ),
        prefer_input_signals: boolean_option(option, "preferInputSignals", true),
        prefer_query_signals: boolean_option(option, "preferQuerySignals", true),
        use_type_checking: boolean_option(option, "useTypeChecking", false),
        additional_signal_creation_functions: option
            .get("additionalSignalCreationFunctions")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect(),
    }
}

fn boolean_option(options: &Value, name: &str, default: bool) -> bool {
    options
        .get(name)
        .and_then(Value::as_bool)
        .unwrap_or(default)
}

fn property_is_signal(
    property: &PropertyDefinition<'_>,
    options: &PreferSignalsOptions<'_>,
    signal_returning_functions: &[CompactString],
) -> bool {
    if let Some(type_annotation) = &property.type_annotation {
        return type_annotation_is_known_signal(&type_annotation.type_annotation);
    }

    let Some(value) = &property.value else {
        return false;
    };
    if initializer_is_known_signal(value, &options.additional_signal_creation_functions) {
        return true;
    }
    options.use_type_checking && expression_has_known_signal_type(value, signal_returning_functions)
}

fn type_annotation_is_known_signal(annotation: &TSType<'_>) -> bool {
    let TSType::TSTypeReference(reference) = annotation else {
        return false;
    };
    if reference.type_arguments.is_none() {
        return false;
    }
    let TSTypeName::IdentifierReference(identifier) = &reference.type_name else {
        return false;
    };
    KNOWN_SIGNAL_TYPES.contains(&identifier.name.as_str())
}

fn initializer_is_known_signal(
    value: &Expression<'_>,
    additional_signal_creation_functions: &[&str],
) -> bool {
    let Expression::CallExpression(call) = value else {
        return false;
    };
    let mut call = call.as_ref();

    if member_identifier_name(&call.callee) == Some("asReadonly") {
        let Some(object) = member_object(&call.callee) else {
            return false;
        };
        let Expression::CallExpression(inner_call) = object else {
            return false;
        };
        call = inner_call.as_ref();
    }

    let callee = match &call.callee {
        Expression::StaticMemberExpression(member) => {
            if member.property.name != "required" {
                return false;
            }
            &member.object
        }
        Expression::ComputedMemberExpression(member) => {
            if matches!(
                &member.expression,
                Expression::Identifier(identifier) if identifier.name != "required"
            ) {
                return false;
            }
            &member.object
        }
        Expression::PrivateFieldExpression(member) => &member.object,
        callee => callee,
    };

    let Expression::Identifier(identifier) = callee else {
        return false;
    };
    KNOWN_SIGNAL_CREATION_FUNCTIONS.contains(&identifier.name.as_str())
        || additional_signal_creation_functions.contains(&identifier.name.as_str())
}

fn expression_has_known_signal_type(
    value: &Expression<'_>,
    signal_returning_functions: &[CompactString],
) -> bool {
    let Expression::CallExpression(call) = value else {
        return false;
    };
    let Expression::Identifier(identifier) = &call.callee else {
        return false;
    };
    signal_returning_functions
        .iter()
        .any(|name| name == identifier.name.as_str())
}

fn member_identifier_name<'a>(expression: &'a Expression<'a>) -> Option<&'a str> {
    match expression {
        Expression::StaticMemberExpression(member) => Some(member.property.name.as_str()),
        Expression::ComputedMemberExpression(member) => {
            let Expression::Identifier(identifier) = &member.expression else {
                return None;
            };
            Some(identifier.name.as_str())
        }
        Expression::PrivateFieldExpression(_) => None,
        _ => None,
    }
}

fn member_object<'a>(expression: &'a Expression<'a>) -> Option<&'a Expression<'a>> {
    match expression {
        Expression::StaticMemberExpression(member) => Some(&member.object),
        Expression::ComputedMemberExpression(member) => Some(&member.object),
        Expression::PrivateFieldExpression(member) => Some(&member.object),
        _ => None,
    }
}

#[cfg(test)]
#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "Pinned upstream fixtures use serde_json values and Vec assertions to mirror the JavaScript ABI."
)]
mod tests {
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use serde_json::{Value, json};

    use super::PREFER_SIGNALS;
    use crate::{Diagnostic, ScanOptions, scan_angular_eslint_with_options};

    const UPSTREAM_FIXTURE: &str =
        include_str!("../../../npm/angular-eslint/test/fixtures/prefer-signals-v22.1.0.json");

    fn scan(source: &str, options: Value) -> Vec<Diagnostic> {
        scan_angular_eslint_with_options(
            source,
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from(PREFER_SIGNALS)]),
                options,
            },
        )
        .into_vec()
    }

    #[test]
    fn replays_every_upstream_authored_valid_case() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid prefer-signals fixture");
        let valid = fixture["valid"]
            .as_array()
            .expect("fixture has valid cases");
        assert_eq!(valid.len(), 39);
        for test_case in valid {
            let diagnostics = scan(
                test_case["code"].as_str().expect("valid case has source"),
                test_case["options"].clone(),
            );
            assert!(
                diagnostics.is_empty(),
                "{}: {diagnostics:#?}",
                test_case["name"].as_str().expect("valid case has name"),
            );
        }
    }

    #[test]
    fn replays_every_upstream_authored_invalid_location_message_and_data() {
        let fixture: Value =
            serde_json::from_str(UPSTREAM_FIXTURE).expect("valid prefer-signals fixture");
        let invalid = fixture["invalid"]
            .as_array()
            .expect("fixture has invalid cases");
        assert_eq!(invalid.len(), 26);
        for test_case in invalid {
            let diagnostics = scan(
                test_case["code"].as_str().expect("invalid case has source"),
                test_case["options"].clone(),
            );
            let errors = test_case["errors"]
                .as_array()
                .expect("invalid case has errors");
            assert_eq!(
                diagnostics.len(),
                errors.len(),
                "{}: {diagnostics:#?}",
                test_case["name"].as_str().expect("invalid case has name"),
            );
            for (diagnostic, error) in diagnostics.iter().zip(errors) {
                assert_eq!(diagnostic.message_id, error["messageId"]);
                assert_eq!(
                    diagnostic.loc,
                    crate::DiagnosticLoc {
                        start_line: error["line"].as_u64().expect("line") as u32,
                        start_column: error["column"].as_u64().expect("column") as u32 - 1,
                        end_line: error["endLine"].as_u64().expect("end line") as u32,
                        end_column: error["endColumn"].as_u64().expect("end column") as u32 - 1,
                    },
                    "{}",
                    test_case["name"].as_str().expect("invalid case has name"),
                );
                let actual_data = diagnostic
                    .data
                    .iter()
                    .map(|datum| (datum.key.as_str(), datum.value.as_str()))
                    .collect::<Vec<_>>();
                let expected_data = error["data"]
                    .as_object()
                    .expect("error data is an object")
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str().expect("string data")))
                    .collect::<Vec<_>>();
                assert_eq!(actual_data, expected_data);
            }
        }
    }

    #[test]
    fn preserves_upstream_member_expression_edges() {
        let source = r#"
class Test {
  computedLiteral = input["anything"]();
  computedRequired = input[required]();
  rejectedComputedIdentifier = input[anything]();
  rejectedStaticMember = input.optional();
  asReadonly = signal(1)[asReadonly]();
}
"#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 3);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.loc.start_line)
                .collect::<Vec<_>>(),
            vec![3, 4, 7],
        );
    }

    #[test]
    fn recognizes_every_known_signal_type_and_requires_upstream_type_shape() {
        let source = r#"
class Test {
  input: InputSignal<number>;
  model: ModelSignal<number>;
  signal: Signal<number>;
  writable: WritableSignal<number>;
  transformed: InputSignalWithTransform<string, number>;
  missingArguments: Signal;
  qualified: core.Signal<number>;
  unrelated: ReadonlySignal<number>;
  readonly alreadySafe: Signal<number>;
}
"#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(diagnostics.len(), 5);
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.loc.start_line)
                .collect::<Vec<_>>(),
            vec![3, 4, 5, 6, 7],
        );
    }

    #[test]
    fn recognizes_every_known_factory_required_readonly_and_custom_form() {
        let source = r#"
class Test {
  a = computed(() => 1);
  b = contentChild("x");
  c = contentChildren("x");
  d = input();
  e = linkedSignal(() => source);
  f = model();
  g = signal(1);
  h = toSignal(source);
  i = viewChild("x");
  j = viewChildren("x");
  k = input.required();
  l = signal(1).asReadonly();
  m = createSignal();
}
"#;
        let diagnostics = scan(
            source,
            json!([{ "additionalSignalCreationFunctions": ["createSignal"] }]),
        );
        assert_eq!(diagnostics.len(), 13);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.message_id == "preferReadonlySignalProperties"),
        );
    }

    #[test]
    fn reports_all_legacy_decorators_with_exact_query_data() {
        let source = r#"
class Test {
  @Input() input: string;
  @ContentChild("x") contentChild: Widget;
  @ContentChildren("x") contentChildren: QueryList<Widget>;
  @ViewChild("x") viewChild: Widget;
  @ViewChildren("x") viewChildren: QueryList<Widget>;
}
"#;
        let diagnostics = scan(source, json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            vec![
                "preferInputSignals",
                "preferQuerySignals",
                "preferQuerySignals",
                "preferQuerySignals",
                "preferQuerySignals",
            ],
        );
        assert_eq!(
            diagnostics[4]
                .data
                .iter()
                .map(|datum| (datum.key.as_str(), datum.value.as_str()))
                .collect::<Vec<_>>(),
            vec![("function", "viewChildren"), ("decorator", "ViewChildren"),],
        );
    }

    #[test]
    fn rejects_strings_comments_namespaced_and_bare_decorator_near_misses() {
        let source = r#"
const text = "@Input() value = signal(1)";
// @ViewChild("x")
class Test {
  @angular.Input() namespaced: string;
  @Input bare: string;
  member = angular.signal(1);
  constructed = new Signal<number>();
  state = useState(0);
}
"#;
        assert!(scan(source, json!([])).is_empty());
    }

    #[test]
    fn uses_defaults_for_partial_and_non_boolean_options() {
        let source = "class Test { @Input() value = signal(1); }";
        assert_eq!(
            scan(
                source,
                json!([{
                    "preferInputSignals": "invalid",
                    "preferQuerySignals": null,
                    "additionalSignalCreationFunctions": [false, 1],
                }]),
            )
            .len(),
            2,
        );
        assert_eq!(scan(source, json!(["invalid"])).len(), 2);
    }

    #[test]
    fn preserves_utf16_columns_for_property_and_decorator_reports() {
        let diagnostics = scan(
            "class Test { emoji = '😀'; @Input() signalValue = signal(1); }",
            json!([]),
        );
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| (
                    diagnostic.message_id,
                    diagnostic.loc.start_column,
                    diagnostic.loc.end_column,
                ))
                .collect::<Vec<_>>(),
            vec![
                ("preferReadonlySignalProperties", 36, 47),
                ("preferInputSignals", 27, 35),
            ],
        );
    }

    #[test]
    fn options_disable_independent_rule_branches() {
        let source = r#"
class Test {
  @Input() signal = signal(1);
  @ViewChild("child") child: Widget;
}
"#;
        let diagnostics = scan(
            source,
            json!([{
                "preferReadonlySignalProperties": false,
                "preferInputSignals": false,
                "preferQuerySignals": false,
            }]),
        );
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn reports_property_before_its_decorator_like_upstream_traversal() {
        let diagnostics = scan("class Test { @Input() value = signal(1); }", json!([]));
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message_id)
                .collect::<Vec<_>>(),
            vec!["preferReadonlySignalProperties", "preferInputSignals"],
        );
    }

    #[test]
    fn stays_isolated_and_fails_closed_on_parse_errors() {
        let other_rule = scan_angular_eslint_with_options(
            "class Test { value = signal(1); }",
            "fixture.ts",
            &ScanOptions {
                rule_names: SmallVec::from_vec(vec![CompactString::from("no-output-rename")]),
                options: json!([]),
            },
        );
        assert!(
            other_rule
                .iter()
                .all(|diagnostic| diagnostic.rule_name != PREFER_SIGNALS)
        );
        assert!(scan("class Test { value = signal(", json!([])).is_empty());
    }
}
