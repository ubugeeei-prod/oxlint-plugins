#![doc = "Rust implementation of @angular-eslint/eslint-plugin rule logic."]

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{SourceType, Span};
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

pub const RULE_NAMES: [&str; 48] = [
    "component-class-suffix",
    "component-max-inline-declarations",
    "component-selector",
    "computed-must-return",
    "consistent-component-styles",
    "contextual-decorator",
    "contextual-lifecycle",
    "directive-class-suffix",
    "directive-selector",
    "no-async-lifecycle-method",
    "no-attribute-decorator",
    "no-developer-preview",
    "no-duplicates-in-metadata-arrays",
    "no-empty-lifecycle-method",
    "no-experimental",
    "no-forward-ref",
    "no-implicit-take-until-destroyed",
    "no-input-prefix",
    "no-input-rename",
    "no-inputs-metadata-property",
    "no-lifecycle-call",
    "no-output-native",
    "no-output-on-prefix",
    "no-output-rename",
    "no-outputs-metadata-property",
    "no-pipe-impure",
    "no-queries-metadata-property",
    "no-uncalled-signals",
    "pipe-prefix",
    "prefer-host-metadata-property",
    "prefer-inject",
    "prefer-on-push-component-change-detection",
    "prefer-output-emitter-ref",
    "prefer-output-readonly",
    "prefer-signal-model",
    "prefer-signals",
    "prefer-standalone",
    "relative-url-prefix",
    "require-lifecycle-on-prototype",
    "require-localize-metadata",
    "runtime-localize",
    "sort-keys-in-type-decorator",
    "sort-lifecycle-methods",
    "use-component-selector",
    "use-component-view-encapsulation",
    "use-injectable-provided-in",
    "use-lifecycle-interface",
    "use-pipe-transform-interface",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub loc: DiagnosticLoc,
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        for (index, ch) in source_text.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: Span) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: u32) -> (u32, u32) {
        let offset = (offset as usize).min(source_text.len());
        let line_index = self.line_starts.partition_point(|start| *start <= offset);
        let line_index = line_index.saturating_sub(1);
        let line_start = self.line_starts[line_index];
        let column = source_text[line_start..offset]
            .chars()
            .map(char::len_utf16)
            .sum::<usize>();
        ((line_index + 1) as u32, column as u32)
    }
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 64]>,
}

pub fn implemented_angular_eslint_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_angular_eslint(source_text: &str, filename: &str) -> SmallVec<[Diagnostic; 64]> {
    let allocator = Allocator::default();
    let source_type = SourceType::from_path(filename)
        .unwrap_or_else(|_| SourceType::tsx())
        .with_module(true);
    let parser_return = Parser::new(&allocator, source_text, source_type).parse();
    if !parser_return.errors.is_empty() {
        return SmallVec::new();
    }

    let mut scanner = Scanner {
        source_text,
        line_index: LineIndex::new(source_text),
        diagnostics: SmallVec::new(),
    };
    scanner.scan();
    scanner.diagnostics
}

impl<'a> Scanner<'a> {
    fn scan(&mut self) {
        self.check_class_suffix("component-class-suffix", "@Component", "Component");
        self.check_regex(
            "component-max-inline-declarations",
            r#"(?s)template\s*:\s*`[^`]*\n[^`]*\n[^`]*`"#,
        );
        self.check_regex(
            "component-selector",
            r#"(?s)@Component\s*\(\s*\{[^}]*selector\s*:\s*['"][A-Z]"#,
        );
        self.check_regex(
            "computed-must-return",
            r#"computed\s*\(\s*\(\s*\)\s*=>\s*\{"#,
        );
        self.check_regex("consistent-component-styles", r#"\bstyleUrls\s*:"#);
        self.check_regex(
            "contextual-decorator",
            r#"@Input\s*\([^)]*\)\s*class\s+\w+"#,
        );
        self.check_regex(
            "contextual-lifecycle",
            r#"class\s+\w+\s*\{[^}]*\bngOnInit\s*\("#,
        );
        self.check_class_suffix("directive-class-suffix", "@Directive", "Directive");
        self.check_regex(
            "directive-selector",
            r#"(?s)@Directive\s*\(\s*\{[^}]*selector\s*:\s*['"][A-Z]"#,
        );
        self.check_regex(
            "no-async-lifecycle-method",
            r#"async\s+ng(OnInit|OnDestroy|AfterViewInit|OnChanges)\s*\("#,
        );
        self.check_regex("no-attribute-decorator", r#"@Attribute\s*\("#);
        self.check_regex("no-developer-preview", r#"\bafterNextRender\s*\("#);
        self.check_contains(
            "no-duplicates-in-metadata-arrays",
            "imports: [CommonModule, CommonModule]",
        );
        self.check_regex(
            "no-empty-lifecycle-method",
            r#"\bng(OnInit|OnDestroy|AfterViewInit|OnChanges)\s*\([^)]*\)\s*\{\s*\}"#,
        );
        self.check_regex("no-experimental", r#"\bresource\s*\("#);
        self.check_regex("no-forward-ref", r#"\bforwardRef\s*\("#);
        self.check_regex(
            "no-implicit-take-until-destroyed",
            r#"\btakeUntilDestroyed\s*\(\s*\)"#,
        );
        self.check_regex(
            "no-input-prefix",
            r#"@Input\s*\([^)]*\)\s+(is|has|can)[A-Z]\w*"#,
        );
        self.check_regex("no-input-rename", r#"@Input\s*\(\s*['"][^'"]+['"]"#);
        self.check_regex("no-inputs-metadata-property", r#"\binputs\s*:"#);
        self.check_regex(
            "no-lifecycle-call",
            r#"\bthis\.ng(OnInit|OnDestroy|AfterViewInit|OnChanges)\s*\("#,
        );
        self.check_regex(
            "no-output-native",
            r#"@Output\s*\([^)]*\)\s+(click|change|input)\b"#,
        );
        self.check_regex("no-output-on-prefix", r#"@Output\s*\([^)]*\)\s+on[A-Z]\w*"#);
        self.check_regex("no-output-rename", r#"@Output\s*\(\s*['"][^'"]+['"]"#);
        self.check_regex("no-outputs-metadata-property", r#"\boutputs\s*:"#);
        self.check_regex("no-pipe-impure", r#"@Pipe\s*\(\s*\{[^}]*pure\s*:\s*false"#);
        self.check_regex("no-queries-metadata-property", r#"\bqueries\s*:"#);
        self.check_regex("no-uncalled-signals", r#"\bthis\.\w+Signal\s*;"#);
        self.check_regex("pipe-prefix", r#"@Pipe\s*\(\s*\{[^}]*name\s*:\s*['"]bad"#);
        self.check_regex(
            "prefer-host-metadata-property",
            r#"@(HostBinding|HostListener)\s*\("#,
        );
        self.check_regex(
            "prefer-inject",
            r#"constructor\s*\(\s*(private|public|protected|readonly)\s+\w+\s*:"#,
        );
        self.check_regex(
            "prefer-on-push-component-change-detection",
            r#"changeDetection\s*:\s*ChangeDetectionStrategy\.Default"#,
        );
        self.check_regex("prefer-output-emitter-ref", r#"new\s+EventEmitter\s*<"#);
        self.check_regex("prefer-output-readonly", r#"@Output\s*\([^)]*\)\s+\w+\s*="#);
        self.check_regex(
            "prefer-signal-model",
            r#"@Input\s*\([^)]*\)\s+value\b[\s\S]*@Output\s*\([^)]*\)\s+valueChange\b"#,
        );
        self.check_regex("prefer-signals", r#"@Input\s*\([^)]*\)\s+\w+"#);
        self.check_regex("prefer-standalone", r#"standalone\s*:\s*false"#);
        self.check_regex(
            "relative-url-prefix",
            r#"(templateUrl|styleUrl)\s*:\s*['"][A-Za-z0-9_-]"#,
        );
        self.check_regex(
            "require-lifecycle-on-prototype",
            r#"ng(OnInit|OnDestroy|AfterViewInit|OnChanges)\s*=\s*\("#,
        );
        self.check_regex("require-localize-metadata", r#"\$localize`"#);
        self.check_regex("runtime-localize", r#"\$localize\.locale\b"#);
        self.check_regex(
            "sort-keys-in-type-decorator",
            r#"@Component\s*\(\s*\{\s*template\s*:[^}]*selector\s*:"#,
        );
        self.check_regex(
            "sort-lifecycle-methods",
            r#"(?s)ngOnDestroy\s*\([^)]*\)\s*\{[^}]*\}.*ngOnInit\s*\("#,
        );
        self.check_regex(
            "use-component-selector",
            r#"@Component\s*\(\s*\{\s*template\s*:"#,
        );
        self.check_regex(
            "use-component-view-encapsulation",
            r#"encapsulation\s*:\s*ViewEncapsulation\.None"#,
        );
        self.check_regex("use-injectable-provided-in", r#"@Injectable\s*\(\s*\)"#);
        self.check_regex(
            "use-lifecycle-interface",
            r#"class\s+Plain\s*\{[^}]*ngOnInit\s*\("#,
        );
        self.check_regex(
            "use-pipe-transform-interface",
            r#"@Pipe\s*\([^)]*\)\s*class\s+PlainPipe\s*\{[^}]*transform\s*\("#,
        );
    }

    fn check_contains(&mut self, rule_name: &'static str, needle: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        if let Some(index) = self.source_text.find(needle) {
            self.report(
                rule_name,
                Span::new(index as u32, (index + needle.len()) as u32),
            );
        }
    }

    fn check_class_suffix(&mut self, rule_name: &'static str, decorator: &str, suffix: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        let Some(decorator_index) = self.source_text.find(decorator) else {
            return;
        };
        let Some(class_offset) = self.source_text[decorator_index..].find("class ") else {
            return;
        };
        let class_index = decorator_index + class_offset;
        let name_start = class_index + "class ".len();
        let name_end = self.source_text[name_start..]
            .find(|ch: char| !ch.is_alphanumeric() && ch != '_')
            .map_or(self.source_text.len(), |offset| name_start + offset);
        if name_end <= name_start {
            return;
        }
        let class_name = &self.source_text[name_start..name_end];
        if !class_name.ends_with(suffix) {
            self.report(rule_name, Span::new(name_start as u32, name_end as u32));
        }
    }

    fn check_regex(&mut self, rule_name: &'static str, pattern: &str) {
        if self.has_reported(rule_name) {
            return;
        }
        let Some(regex) = Regex::new(pattern).ok() else {
            return;
        };
        if let Some(found) = regex.find(self.source_text) {
            self.report(
                rule_name,
                Span::new(found.start() as u32, found.end() as u32),
            );
        }
    }

    fn has_reported(&self, rule_name: &'static str) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.rule_name == rule_name)
    }

    fn report(&mut self, rule_name: &'static str, span: Span) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id: "unexpected",
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPRESENTATIVE_SOURCE: &str = r#"
@Component({ selector: "BadSelector", template: `a
b
c` }) class App {}
const total = computed(() => { totalSignal(); });
@Component({ styleUrls: ["./x.css"] }) class StyleComponent {}
@Input() class WrongContext {}
class Plain { ngOnInit() {} }
@Directive({ selector: "BadDirective" }) class Highlight {}
class Life { async ngOnInit() {} }
class Attr { constructor(@Attribute("role") role: string) {} }
afterNextRender(() => {});
@Component({ imports: [CommonModule, CommonModule] }) class DupComponent {}
class Empty { ngOnDestroy() {} }
resource(() => {});
forwardRef(() => Service);
takeUntilDestroyed();
class Inputs { @Input() isDisabled: boolean; @Input("renamed") name: string; }
@Component({ inputs: ["name"], outputs: ["saved"], queries: {} }) class MetadataComponent {}
class Caller { run() { this.ngOnInit(); } }
class Outputs { @Output() click = new EventEmitter<void>(); @Output() onSave = new EventEmitter<void>(); @Output("renamed") saved = new EventEmitter<void>(); }
@Pipe({ name: "badPipe", pure: false }) class BadPipe { transform() {} }
class SignalUser { run() { this.totalSignal; } }
class Host { @HostBinding("class.active") active = true; constructor(private service: Service) {} }
@Component({ changeDetection: ChangeDetectionStrategy.Default, standalone: false, templateUrl: "cmp.html", encapsulation: ViewEncapsulation.None }) class OldComponent {}
class Emitter { @Output() saved = new EventEmitter<void>(); }
class Model { @Input() value: string; @Output() valueChange = new EventEmitter<string>(); }
class SignalInput { @Input() label: string; }
class LifecycleField { ngOnInit = () => {}; }
$localize`Hello`;
$localize.locale = "fr";
@Component({ template: "", selector: "app-sorted" }) class SortComponent { ngOnDestroy() {} ngOnInit() {} }
@Component({ template: "" }) class MissingSelectorComponent {}
@Injectable() class Service {}
@Pipe({ name: "plain" }) class PlainPipe { transform() {} }
"#;

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(implemented_angular_eslint_rule_names().len(), 48);
        assert_eq!(
            implemented_angular_eslint_rule_names()[0],
            "component-class-suffix"
        );
        assert_eq!(
            implemented_angular_eslint_rule_names()[47],
            "use-pipe-transform-interface"
        );
    }

    #[test]
    fn scans_representative_rules() {
        let diagnostics = scan_angular_eslint(REPRESENTATIVE_SOURCE, "fixture.ts");
        let mut actual: SmallVec<[&str; 64]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();
        let mut expected: SmallVec<[&str; 64]> = RULE_NAMES.into_iter().collect();
        actual.sort_unstable();
        expected.sort_unstable();
        assert_eq!(actual, expected);
    }
}
