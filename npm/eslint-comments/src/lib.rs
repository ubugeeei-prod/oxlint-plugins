//! NAPI boundary for the eslint-comments oxlint plugin.
//!
//! The JavaScript wrapper passes every comment in a file once per rule, and the
//! Rust core returns the diagnostics to report. This keeps work batched at the
//! file level instead of one NAPI call per AST node.

pub use napi_abi::{
    CommentInput, Diagnostic, DiagnosticData, DiagnosticLoc, PositionInput, ProblemInput,
    scan_disable_enable_pair, scan_no_aggregating_enable, scan_no_duplicate_disable,
    scan_no_restricted_disable, scan_no_unlimited_disable, scan_no_unused_disable,
    scan_no_unused_enable, scan_no_use, scan_require_description,
};

#[allow(
    clippy::disallowed_macros,
    clippy::disallowed_types,
    reason = "NAPI public ABI and proc macros require String/Vec/format expansion; values are converted into carton/core types before rule logic runs."
)]
mod napi_abi {
    use napi_derive::napi;
    use oxlint_plugins_eslint_comments::directive::CommentKind;
    use oxlint_plugins_eslint_comments::{
        Comment, Diagnostic as CoreDiagnostic, Location, Position, Problem, disable_enable_pair,
        no_aggregating_enable, no_duplicate_disable, no_restricted_disable, no_unlimited_disable,
        no_unused_disable, no_unused_enable, no_use, require_description,
    };

    /// A comment token, as collected from `sourceCode.getAllComments()`.
    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct CommentInput {
        /// `"Line"` or `"Block"`.
        pub kind: String,
        /// The comment body (without the `//` or `/* */` delimiters).
        pub value: String,
        pub start_line: u32,
        pub start_column: i32,
        pub end_line: u32,
        pub end_column: i32,
    }

    /// A source position passed from the wrapper (e.g. the first token).
    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct PositionInput {
        pub line: u32,
        pub column: i32,
    }

    /// A lint problem from `sourceCode.getDisableDirectives().problems`.
    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct ProblemInput {
        pub rule_id: Option<String>,
        pub line: u32,
        pub column: i32,
    }

    /// Values interpolated into a diagnostic message template.
    #[napi(object)]
    #[derive(Clone, Debug, Default)]
    pub struct DiagnosticData {
        pub kind: Option<String>,
        pub rule_id: Option<String>,
        pub count: Option<u32>,
    }

    /// A report location. A `column` of `-1` means "force the whole line".
    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct DiagnosticLoc {
        pub start_line: u32,
        pub start_column: i32,
        pub end_line: u32,
        pub end_column: i32,
    }

    /// A diagnostic the wrapper maps onto `context.report`.
    #[napi(object)]
    #[derive(Clone, Debug)]
    pub struct Diagnostic {
        pub message_id: String,
        pub data: DiagnosticData,
        pub loc: DiagnosticLoc,
    }

    /// `no-unlimited-disable`: report `eslint-disable*` comments without rule names.
    #[napi]
    pub fn scan_no_unlimited_disable(comments: Vec<CommentInput>) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        no_unlimited_disable(&core)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-use`: report directive comments whose kind is not in `allow`.
    #[napi]
    pub fn scan_no_use(comments: Vec<CommentInput>, allow: Vec<String>) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        let allowed: Vec<&str> = allow.iter().map(String::as_str).collect();
        no_use(&core, &allowed)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `require-description`: report directive comments without a description.
    #[napi]
    pub fn scan_require_description(
        comments: Vec<CommentInput>,
        ignore: Vec<String>,
    ) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        let ignored: Vec<&str> = ignore.iter().map(String::as_str).collect();
        require_description(&core, &ignored)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-aggregating-enable`: report enables that close multiple disables.
    #[napi]
    pub fn scan_no_aggregating_enable(comments: Vec<CommentInput>) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        no_aggregating_enable(&core)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-duplicate-disable`: report disables that duplicate an active disable.
    #[napi]
    pub fn scan_no_duplicate_disable(comments: Vec<CommentInput>) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        no_duplicate_disable(&core)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-unused-enable`: report enables that close no open disable.
    #[napi]
    pub fn scan_no_unused_enable(comments: Vec<CommentInput>) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        no_unused_enable(&core)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-restricted-disable`: report disables of rules matching the patterns.
    #[napi]
    pub fn scan_no_restricted_disable(
        comments: Vec<CommentInput>,
        patterns: Vec<String>,
    ) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        let pattern_refs: Vec<&str> = patterns.iter().map(String::as_str).collect();
        no_restricted_disable(&core, &pattern_refs)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `no-unused-disable`: report disables that suppress none of `problems`.
    #[napi]
    pub fn scan_no_unused_disable(
        comments: Vec<CommentInput>,
        problems: Vec<ProblemInput>,
    ) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        let core_problems: Vec<Problem> = problems
            .iter()
            .map(|problem| Problem {
                rule_id: problem.rule_id.as_deref(),
                position: Position {
                    line: problem.line,
                    column: problem.column,
                },
            })
            .collect();
        no_unused_disable(&core, &core_problems)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    /// `disable-enable-pair`: report disabled areas with no matching enable.
    #[napi]
    pub fn scan_disable_enable_pair(
        comments: Vec<CommentInput>,
        allow_whole_file: bool,
        first_token_start: Option<PositionInput>,
    ) -> Vec<Diagnostic> {
        let core = to_core_comments(&comments);
        let first = first_token_start.map(|position| Position {
            line: position.line,
            column: position.column,
        });
        disable_enable_pair(&core, allow_whole_file, first)
            .into_iter()
            .map(diagnostic_from_core)
            .collect()
    }

    fn to_core_comments(comments: &[CommentInput]) -> Vec<Comment<'_>> {
        comments
            .iter()
            .map(|comment| Comment {
                kind: if comment.kind == "Line" {
                    CommentKind::Line
                } else {
                    CommentKind::Block
                },
                value: comment.value.as_str(),
                loc: Location {
                    start: Position {
                        line: comment.start_line,
                        column: comment.start_column,
                    },
                    end: Position {
                        line: comment.end_line,
                        column: comment.end_column,
                    },
                },
            })
            .collect()
    }

    fn diagnostic_from_core(diagnostic: CoreDiagnostic) -> Diagnostic {
        Diagnostic {
            message_id: diagnostic.message_id.into_string(),
            data: DiagnosticData {
                kind: diagnostic.data.kind.map(|value| value.into_string()),
                rule_id: diagnostic.data.rule_id.map(|value| value.into_string()),
                count: diagnostic.data.count,
            },
            loc: DiagnosticLoc {
                start_line: diagnostic.loc.start.line,
                start_column: diagnostic.loc.start.column,
                end_line: diagnostic.loc.end.line,
                end_column: diagnostic.loc.end.column,
            },
        }
    }
}
