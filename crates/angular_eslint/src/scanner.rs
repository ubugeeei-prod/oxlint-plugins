//! Regex-driven scanner for the angular-eslint port.

use oxc_span::Span;
use oxlint_plugins_carton::SmallVec;
use regex::Regex;

use crate::types::{Diagnostic, LineIndex};

pub(crate) struct Scanner<'a> {
    pub(crate) source_text: &'a str,
    pub(crate) line_index: LineIndex,
    pub(crate) diagnostics: SmallVec<[Diagnostic; 64]>,
}

impl<'a> Scanner<'a> {
    pub(crate) fn scan(&mut self) {
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
