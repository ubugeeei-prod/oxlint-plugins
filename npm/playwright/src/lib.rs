//! NAPI boundary for the playwright oxlint plugin.

pub use napi_abi::{
    Diagnostic, DiagnosticData, DiagnosticFix, DiagnosticLoc, PlaywrightHookAlias,
    PlaywrightRestriction, PlaywrightScanOptions, PlaywrightTagPattern, PlaywrightTitlePattern,
    PlaywrightTitlePatternOptions, PlaywrightValidTestTagsOptions, PlaywrightValidTitleOptions,
    implemented_playwright_rule_names, scan_playwright,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI requires String/Vec; values are converted before returning to JavaScript."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_carton::{CompactString, SmallVec};
    use oxlint_plugins_playwright as core;

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PlaywrightRestriction {
        pub value: String,
        pub message: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PlaywrightHookAlias {
        pub name: String,
        pub hook_name: String,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PlaywrightScanOptions {
        pub allowed_hooks: Option<Vec<String>>,
        pub allowed_prefixes: Option<Vec<String>>,
        pub assert_function_names: Option<Vec<String>>,
        pub assert_function_patterns: Option<Vec<String>>,
        pub ignore: Option<Vec<String>>,
        pub ignore_top_level_describe: Option<bool>,
        pub hook_aliases: Option<Vec<PlaywrightHookAlias>>,
        pub restricted_locators: Option<Vec<PlaywrightRestriction>>,
        pub restricted_matchers: Option<Vec<PlaywrightRestriction>>,
        pub restricted_roles: Option<Vec<PlaywrightRestriction>>,
        pub expect_aliases: Option<Vec<String>>,
        pub test_aliases: Option<Vec<String>>,
        pub valid_title: Option<PlaywrightValidTitleOptions>,
        pub valid_test_tags: Option<PlaywrightValidTestTagsOptions>,
        pub max_expects: Option<u32>,
        pub max_nested_describe: Option<u32>,
        pub max_top_level_describes: Option<f64>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PlaywrightValidTitleOptions {
        pub disallowed_words: Option<Vec<String>>,
        pub ignore_spaces: Option<bool>,
        pub ignore_type_of_describe_name: Option<bool>,
        pub ignore_type_of_step_name: Option<bool>,
        pub ignore_type_of_test_name: Option<bool>,
        pub must_match: Option<PlaywrightTitlePatternOptions>,
        pub must_not_match: Option<PlaywrightTitlePatternOptions>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PlaywrightTitlePatternOptions {
        pub describe: Option<PlaywrightTitlePattern>,
        pub step: Option<PlaywrightTitlePattern>,
        pub test: Option<PlaywrightTitlePattern>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PlaywrightTitlePattern {
        pub source: String,
        pub message: Option<String>,
    }

    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct PlaywrightValidTestTagsOptions {
        pub allowed_tags: Option<Vec<PlaywrightTagPattern>>,
        pub disallowed_tags: Option<Vec<PlaywrightTagPattern>>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PlaywrightTagPattern {
        pub source: String,
        pub flags: String,
        pub is_regex: bool,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: u32,
        pub end_line: u32,
        pub end_column: u32,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub rule_name: String,
        pub message_id: String,
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
        pub fix: Option<DiagnosticFix>,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticFix {
        pub start: u32,
        pub end: u32,
        pub replacement: String,
    }

    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticData {
        pub message: String,
        pub amount: Option<String>,
        pub count: Option<String>,
        pub depth: Option<String>,
        pub hook_name: Option<String>,
        pub max: Option<String>,
        pub method: Option<String>,
        pub restriction: Option<String>,
        pub role: Option<String>,
        pub function_name: Option<String>,
        pub pattern: Option<String>,
        pub tag: Option<String>,
        pub word: Option<String>,
        pub s: Option<String>,
    }

    #[napi]
    pub fn implemented_playwright_rule_names() -> Vec<String> {
        core::implemented_playwright_rule_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect()
    }

    #[napi]
    pub fn scan_playwright(
        source_text: String,
        filename: String,
        options: Option<PlaywrightScanOptions>,
    ) -> Vec<Diagnostic> {
        let options = options.unwrap_or_default();
        let valid_title = compact_valid_title(options.valid_title);
        let valid_test_tags = compact_valid_test_tags(options.valid_test_tags);
        let core_options = core::PlaywrightOptions {
            allowed_hooks: compact_strings(options.allowed_hooks),
            allowed_title_prefixes: compact_strings(options.allowed_prefixes),
            assert_function_names: compact_strings(options.assert_function_names),
            assert_function_patterns: compact_strings(options.assert_function_patterns),
            lowercase_title_ignored_methods: compact_strings(options.ignore),
            ignore_top_level_describe: options.ignore_top_level_describe.unwrap_or(false),
            hook_aliases: options
                .hook_aliases
                .unwrap_or_default()
                .into_iter()
                .map(|alias| core::HookAlias {
                    name: CompactString::from(alias.name),
                    hook_name: CompactString::from(alias.hook_name),
                })
                .collect(),
            restricted_locators: compact_restrictions(options.restricted_locators),
            restricted_matchers: compact_restrictions(options.restricted_matchers),
            restricted_roles: compact_restrictions(options.restricted_roles),
            expect_aliases: options
                .expect_aliases
                .unwrap_or_default()
                .into_iter()
                .map(CompactString::from)
                .collect(),
            test_aliases: options
                .test_aliases
                .unwrap_or_default()
                .into_iter()
                .map(CompactString::from)
                .collect(),
            valid_title,
            valid_test_tags,
            max_expects: options.max_expects.filter(|max| *max >= 1).unwrap_or(5),
            max_nested_describe: options.max_nested_describe.unwrap_or(5),
            max_top_level_describes: options
                .max_top_level_describes
                .filter(|max| max.is_finite() && *max >= 1.0),
        };
        core::scan_playwright_with_options(&source_text, &filename, &core_options)
            .into_iter()
            .map(|diagnostic| Diagnostic {
                rule_name: diagnostic.rule_name.to_owned(),
                message_id: diagnostic.message_id.to_owned(),
                data: DiagnosticData {
                    message: diagnostic.data.message.into_string(),
                    amount: diagnostic.data.amount.map(CompactString::into_string),
                    count: diagnostic.data.count.map(CompactString::into_string),
                    depth: diagnostic.data.depth.map(CompactString::into_string),
                    hook_name: diagnostic.data.hook_name.map(CompactString::into_string),
                    max: diagnostic.data.max.map(CompactString::into_string),
                    method: diagnostic.data.method.map(CompactString::into_string),
                    restriction: diagnostic.data.restriction.map(CompactString::into_string),
                    role: diagnostic.data.role.map(CompactString::into_string),
                    function_name: diagnostic
                        .data
                        .function_name
                        .map(CompactString::into_string),
                    pattern: diagnostic.data.pattern.map(CompactString::into_string),
                    tag: diagnostic.data.tag.map(CompactString::into_string),
                    word: diagnostic.data.word.map(CompactString::into_string),
                    s: diagnostic.data.s.map(CompactString::into_string),
                },
                loc: DiagnosticLoc {
                    start_line: diagnostic.loc.start_line,
                    start_column: diagnostic.loc.start_column,
                    end_line: diagnostic.loc.end_line,
                    end_column: diagnostic.loc.end_column,
                },
                fix: diagnostic.fix.map(|fix| DiagnosticFix {
                    start: fix.start,
                    end: fix.end,
                    replacement: fix.replacement.into_string(),
                }),
            })
            .collect()
    }

    fn compact_restrictions(
        restrictions: Option<Vec<PlaywrightRestriction>>,
    ) -> SmallVec<[core::Restriction; 8]> {
        restrictions
            .unwrap_or_default()
            .into_iter()
            .map(|restriction| core::Restriction {
                value: CompactString::from(restriction.value),
                message: restriction.message.map(CompactString::from),
            })
            .collect()
    }

    fn compact_valid_title(
        options: Option<PlaywrightValidTitleOptions>,
    ) -> core::ValidTitleOptions {
        let Some(options) = options else {
            return core::ValidTitleOptions::default();
        };
        core::ValidTitleOptions {
            disallowed_words: options
                .disallowed_words
                .unwrap_or_default()
                .into_iter()
                .map(CompactString::from)
                .collect(),
            ignore_spaces: options.ignore_spaces.unwrap_or(false),
            ignore_type_of_describe_name: options.ignore_type_of_describe_name.unwrap_or(false),
            ignore_type_of_step_name: options.ignore_type_of_step_name.unwrap_or(true),
            ignore_type_of_test_name: options.ignore_type_of_test_name.unwrap_or(false),
            must_match: compact_title_patterns(options.must_match),
            must_not_match: compact_title_patterns(options.must_not_match),
        }
    }

    fn compact_title_patterns(
        patterns: Option<PlaywrightTitlePatternOptions>,
    ) -> core::TitlePatternOptions {
        let Some(patterns) = patterns else {
            return core::TitlePatternOptions::default();
        };
        core::TitlePatternOptions {
            describe: patterns.describe.map(compact_title_pattern),
            step: patterns.step.map(compact_title_pattern),
            test: patterns.test.map(compact_title_pattern),
        }
    }

    fn compact_title_pattern(pattern: PlaywrightTitlePattern) -> core::TitlePattern {
        core::TitlePattern {
            source: CompactString::from(pattern.source),
            message: pattern.message.map(CompactString::from),
        }
    }

    fn compact_valid_test_tags(
        options: Option<PlaywrightValidTestTagsOptions>,
    ) -> core::ValidTestTagsOptions {
        let Some(options) = options else {
            return core::ValidTestTagsOptions::default();
        };
        core::ValidTestTagsOptions {
            allowed_tags: compact_tag_patterns(options.allowed_tags),
            disallowed_tags: compact_tag_patterns(options.disallowed_tags),
        }
    }

    fn compact_tag_patterns(
        patterns: Option<Vec<PlaywrightTagPattern>>,
    ) -> SmallVec<[core::TagPattern; 8]> {
        patterns
            .unwrap_or_default()
            .into_iter()
            .map(|pattern| core::TagPattern {
                source: CompactString::from(pattern.source),
                flags: CompactString::from(pattern.flags),
                is_regex: pattern.is_regex,
            })
            .collect()
    }

    fn compact_strings(values: Option<Vec<String>>) -> SmallVec<[CompactString; 4]> {
        values
            .unwrap_or_default()
            .into_iter()
            .map(CompactString::from)
            .collect()
    }
}
