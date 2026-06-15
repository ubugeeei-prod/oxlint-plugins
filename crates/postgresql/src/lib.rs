#![doc = "Rust implementation of eslint-plugin-postgresql rule logic."]

use oxc_span::Span;
use oxlint_plugins_carton::{CompactString, SmallVec};

pub const RULE_NAMES: [&str; 31] = [
    "consistent-identity-over-serial",
    "consistent-jsonb-over-json",
    "consistent-text-over-varchar",
    "consistent-timestamptz",
    "no-char-type",
    "no-cluster",
    "no-create-role",
    "no-cross-join",
    "no-drop-database",
    "no-drop-schema-cascade",
    "no-drop-table-cascade",
    "no-equality-with-null",
    "no-grant-all",
    "no-grant-to-public",
    "no-money-type",
    "no-natural-join",
    "no-not-in-subquery",
    "no-select-into",
    "no-select-star",
    "no-set-search-path",
    "no-temporary-table",
    "no-time-type",
    "no-truncate-cascade",
    "no-unlogged-table",
    "no-vacuum-full",
    "prefer-cast-operator",
    "prefer-current-timestamp-over-now",
    "prefer-not-equals-operator",
    "require-trailing-semicolon",
    "require-where-in-delete",
    "require-where-in-update",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Style {
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NotEqualsOperator {
    Angle,
    Bang,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CastForm {
    Operator,
    Function,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanOptions {
    pub identity_style: Style,
    pub jsonb_style: Style,
    pub text_style: Style,
    pub timestamptz_style: Style,
    pub not_equals_operator: NotEqualsOperator,
    pub cast_form: CastForm,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            identity_style: Style::Always,
            jsonb_style: Style::Always,
            text_style: Style::Always,
            timestamptz_style: Style::Always,
            not_equals_operator: NotEqualsOperator::Angle,
            cast_form: CastForm::Operator,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub op: Option<CompactString>,
    pub type_name: Option<CompactString>,
}

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
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TokenKind {
    Word,
    Number,
    Symbol,
    String,
    QuotedIdentifier,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Token {
    kind: TokenKind,
    text: CompactString,
    span: Span,
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

pub fn implemented_postgresql_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_postgresql(
    source_text: &str,
    _filename: &str,
    options: &ScanOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let tokens = lex_sql(source_text);
    if tokens.is_empty() {
        return SmallVec::new();
    }

    let line_index = LineIndex::new(source_text);
    let mut scanner = Scanner {
        source_text,
        line_index,
        diagnostics: SmallVec::new(),
        tokens,
        options,
    };
    scanner.scan();
    scanner.diagnostics.sort_by(|left, right| {
        (
            left.loc.start_line,
            left.loc.start_column,
            left.rule_name,
            left.message_id,
        )
            .cmp(&(
                right.loc.start_line,
                right.loc.start_column,
                right.rule_name,
                right.message_id,
            ))
    });
    scanner.diagnostics
}

struct Scanner<'a> {
    source_text: &'a str,
    line_index: LineIndex,
    diagnostics: SmallVec<[Diagnostic; 32]>,
    tokens: SmallVec<[Token; 256]>,
    options: &'a ScanOptions,
}

impl Scanner<'_> {
    fn scan(&mut self) {
        self.scan_trailing_semicolon();
        self.scan_token_patterns();
        self.scan_statements();
    }

    fn scan_trailing_semicolon(&mut self) {
        let Some(last) = self.tokens.last() else {
            return;
        };
        if !last.is_symbol(";") {
            self.report("require-trailing-semicolon", "missingSemicolon", last.span);
        }
    }

    fn scan_token_patterns(&mut self) {
        let len = self.tokens.len();
        for index in 0..len {
            self.scan_select_star(index);
            self.scan_types(index);
            self.scan_binary_operator(index);
            self.scan_current_time_functions(index);
            self.scan_cast_operator(index);
        }
    }

    fn scan_statements(&mut self) {
        let mut start = 0;
        while start < self.tokens.len() {
            while start < self.tokens.len() && self.tokens[start].is_symbol(";") {
                start += 1;
            }
            if start >= self.tokens.len() {
                break;
            }
            let mut end = start;
            while end < self.tokens.len() && !self.tokens[end].is_symbol(";") {
                end += 1;
            }
            self.scan_statement(start, end);
            start = end.saturating_add(1);
        }
    }

    fn scan_statement(&mut self, start: usize, end: usize) {
        if start >= end {
            return;
        }
        self.scan_statement_head(start, end);
        self.scan_statement_contains(start, end);
    }

    fn scan_statement_head(&mut self, start: usize, end: usize) {
        if self.is_word(start, "DROP") && self.is_word(start + 1, "DATABASE") {
            self.report_statement("no-drop-database", "noDropDatabase", start, end);
        }
        if self.is_word(start, "VACUUM") && self.is_word(start + 1, "FULL") {
            self.report_statement("no-vacuum-full", "noVacuumFull", start, end);
        }
        if self.is_word(start, "CLUSTER") {
            self.report_statement("no-cluster", "noCluster", start, end);
        }
        if self.is_word(start, "CREATE")
            && (self.is_word(start + 1, "ROLE") || self.is_word(start + 1, "USER"))
        {
            self.report_statement("no-create-role", "noCreateRole", start, end);
        }
        if self.is_word(start, "UPDATE") && !self.statement_contains_word(start, end, "WHERE") {
            self.report_statement("require-where-in-update", "missingWhere", start, end);
        }
        if self.is_word(start, "DELETE") && !self.statement_contains_word(start, end, "WHERE") {
            self.report_statement("require-where-in-delete", "missingWhere", start, end);
        }
    }

    fn scan_statement_contains(&mut self, start: usize, end: usize) {
        for index in start..end {
            if self.is_word(index, "DROP")
                && self.is_word(index + 1, "SCHEMA")
                && self.statement_contains_word(index + 2, end, "CASCADE")
            {
                self.report_statement("no-drop-schema-cascade", "noDropSchemaCascade", index, end);
            }
            if self.is_word(index, "DROP")
                && self.is_word(index + 1, "TABLE")
                && self.statement_contains_word(index + 2, end, "CASCADE")
            {
                self.report_statement("no-drop-table-cascade", "noCascade", index, end);
            }
            if self.is_word(index, "TRUNCATE")
                && self.statement_contains_word(index + 1, end, "CASCADE")
            {
                self.report_statement("no-truncate-cascade", "noCascade", index, end);
            }
            if self.is_word(index, "GRANT") && self.is_word(index + 1, "ALL") {
                self.report_statement("no-grant-all", "noGrantAll", index, end);
            }
            if self.is_word(index, "GRANT")
                && self.statement_contains_sequence(index + 1, end, &["TO", "PUBLIC"])
            {
                self.report_statement("no-grant-to-public", "noPublic", index, end);
            }
            if self.is_word(index, "SET")
                && (self.is_word(index + 1, "search_path")
                    || ((self.is_word(index + 1, "LOCAL") || self.is_word(index + 1, "SESSION"))
                        && self.is_word(index + 2, "search_path")))
            {
                self.report_statement("no-set-search-path", "noSetSearchPath", index, end);
            }
            if self.is_word(index, "CREATE")
                && (self.is_word(index + 1, "TEMP") || self.is_word(index + 1, "TEMPORARY"))
                && self.is_word(index + 2, "TABLE")
            {
                self.report(
                    "no-temporary-table",
                    "noTemporaryTable",
                    self.tokens[index + 1].span,
                );
            }
            if self.is_word(index, "CREATE")
                && self.is_word(index + 1, "UNLOGGED")
                && self.is_word(index + 2, "TABLE")
            {
                self.report(
                    "no-unlogged-table",
                    "noUnloggedTable",
                    self.tokens[index + 1].span,
                );
            }
            if self.is_word(index, "SELECT") {
                self.scan_select_into(index, end);
            }
        }
    }

    fn scan_select_star(&mut self, select_index: usize) {
        if !self.is_word(select_index, "SELECT") {
            return;
        }
        let mut depth = 0_u32;
        let mut index = select_index + 1;
        while index < self.tokens.len() {
            let token = &self.tokens[index];
            if token.is_symbol(";") {
                return;
            }
            if depth == 0 && token.is_word("FROM") {
                return;
            }
            if token.is_symbol("(") {
                depth += 1;
            } else if token.is_symbol(")") {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && token.is_symbol("*") {
                self.report("no-select-star", "noSelectStar", token.span);
            }
            index += 1;
        }
    }

    fn scan_select_into(&mut self, select_index: usize, end: usize) {
        let mut depth = 0_u32;
        for index in select_index + 1..end {
            let token = &self.tokens[index];
            if token.is_symbol("(") {
                depth += 1;
            } else if token.is_symbol(")") {
                depth = depth.saturating_sub(1);
            } else if depth == 0 && token.is_word("FROM") {
                return;
            } else if depth == 0 && token.is_word("INTO") {
                self.report_statement("no-select-into", "noSelectInto", select_index, end);
                return;
            }
        }
    }

    fn scan_types(&mut self, index: usize) {
        if self.is_word(index, "char") || self.is_word(index, "bpchar") {
            self.report("no-char-type", "noChar", self.tokens[index].span);
        }
        if self.is_word(index, "money") {
            self.report("no-money-type", "noMoney", self.tokens[index].span);
        }
        if self.is_word(index, "time") && !self.previous_is(index, "CURRENT") {
            self.report_type_span("no-time-type", "noTimeType", index);
        }
        if self.is_word(index, "timetz") {
            self.report("no-time-type", "noTimeType", self.tokens[index].span);
        }
        if self.options.jsonb_style == Style::Always && self.is_word(index, "json") {
            self.report(
                "consistent-jsonb-over-json",
                "preferJsonb",
                self.tokens[index].span,
            );
        } else if self.options.jsonb_style == Style::Never && self.is_word(index, "jsonb") {
            self.report(
                "consistent-jsonb-over-json",
                "unexpectedJsonb",
                self.tokens[index].span,
            );
        }
        if self.options.text_style == Style::Always
            && self.is_word(index, "varchar")
            && self
                .token(index + 1)
                .is_some_and(|token| token.is_symbol("("))
        {
            self.report(
                "consistent-text-over-varchar",
                "preferText",
                self.tokens[index].span,
            );
        } else if self.options.text_style == Style::Never && self.is_word(index, "text") {
            self.report(
                "consistent-text-over-varchar",
                "unexpectedText",
                self.tokens[index].span,
            );
        }
        if self.options.timestamptz_style == Style::Always
            && self.is_word(index, "timestamp")
            && !self.timestamp_has_time_zone(index)
            && !self.previous_is(index, "CURRENT")
        {
            self.report(
                "consistent-timestamptz",
                "preferTimestamptz",
                self.tokens[index].span,
            );
        } else if self.options.timestamptz_style == Style::Never
            && (self.is_word(index, "timestamptz") || self.timestamp_has_time_zone(index))
        {
            self.report(
                "consistent-timestamptz",
                "unexpectedTimestamptz",
                self.tokens[index].span,
            );
        }
        if self.options.identity_style == Style::Always
            && (self.is_word(index, "serial")
                || self.is_word(index, "bigserial")
                || self.is_word(index, "smallserial"))
        {
            self.report_with_data(
                "consistent-identity-over-serial",
                "preferIdentity",
                self.tokens[index].span,
                DiagnosticData {
                    type_name: Some(self.tokens[index].text.to_ascii_lowercase()),
                    ..DiagnosticData::default()
                },
            );
        } else if self.options.identity_style == Style::Never
            && self.is_word(index, "GENERATED")
            && self.statement_contains_sequence(index + 1, self.tokens.len(), &["AS", "IDENTITY"])
        {
            self.report(
                "consistent-identity-over-serial",
                "unexpectedIdentity",
                self.tokens[index].span,
            );
        }
    }

    fn scan_binary_operator(&mut self, index: usize) {
        if self.tokens[index].is_symbol("!=") || self.tokens[index].is_symbol("<>") {
            let op = self.tokens[index].text.clone();
            if self
                .token(index + 1)
                .is_some_and(|token| token.is_word("NULL"))
                || index > 0 && self.tokens[index - 1].is_word("NULL")
            {
                self.report_with_data(
                    "no-equality-with-null",
                    "useIsNull",
                    self.tokens[index].span,
                    DiagnosticData {
                        op: Some(op.clone()),
                        ..DiagnosticData::default()
                    },
                );
            }
            let wrong = match self.options.not_equals_operator {
                NotEqualsOperator::Angle => "!=",
                NotEqualsOperator::Bang => "<>",
            };
            if self.tokens[index].is_symbol(wrong) {
                let message_id = match self.options.not_equals_operator {
                    NotEqualsOperator::Angle => "preferAngle",
                    NotEqualsOperator::Bang => "preferBang",
                };
                self.report(
                    "prefer-not-equals-operator",
                    message_id,
                    self.tokens[index].span,
                );
            }
        } else if self.tokens[index].is_symbol("=")
            && (self
                .token(index + 1)
                .is_some_and(|token| token.is_word("NULL"))
                || index > 0 && self.tokens[index - 1].is_word("NULL"))
        {
            self.report_with_data(
                "no-equality-with-null",
                "useIsNull",
                self.tokens[index].span,
                DiagnosticData {
                    op: Some("=".into()),
                    ..DiagnosticData::default()
                },
            );
        }
        if self.is_word(index, "CROSS") && self.is_word(index + 1, "JOIN") {
            self.report_span("no-cross-join", "noCrossJoin", index, index + 1);
        }
        if self.is_word(index, "NATURAL") && self.is_word(index + 1, "JOIN") {
            self.report_span("no-natural-join", "noNaturalJoin", index, index + 1);
        }
        if self.is_word(index, "NOT")
            && self.is_word(index + 1, "IN")
            && self
                .token(index + 2)
                .is_some_and(|token| token.is_symbol("("))
            && self.is_word(index + 3, "SELECT")
        {
            self.report_span("no-not-in-subquery", "noNotInSubquery", index, index + 3);
        }
    }

    fn scan_current_time_functions(&mut self, index: usize) {
        if self.is_word(index, "now")
            && self
                .token(index + 1)
                .is_some_and(|token| token.is_symbol("("))
            && self
                .token(index + 2)
                .is_some_and(|token| token.is_symbol(")"))
        {
            self.report_span(
                "prefer-current-timestamp-over-now",
                "preferCurrentTimestamp",
                index,
                index + 2,
            );
        }
        if self.is_word(index, "localtimestamp") {
            self.report(
                "prefer-current-timestamp-over-now",
                "preferCurrentTimestampOverLocal",
                self.tokens[index].span,
            );
        }
        if self.is_word(index, "localtime") {
            self.report(
                "prefer-current-timestamp-over-now",
                "preferCurrentTimeOverLocal",
                self.tokens[index].span,
            );
        }
    }

    fn scan_cast_operator(&mut self, index: usize) {
        if self.options.cast_form == CastForm::Operator && self.is_word(index, "CAST") {
            self.report(
                "prefer-cast-operator",
                "preferOperator",
                self.tokens[index].span,
            );
        } else if self.options.cast_form == CastForm::Function && self.tokens[index].is_symbol("::")
        {
            self.report(
                "prefer-cast-operator",
                "preferFunction",
                self.tokens[index].span,
            );
        }
    }

    fn report_type_span(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        index: usize,
    ) {
        let end = if self.is_word(index + 1, "WITH")
            && self.is_word(index + 2, "TIME")
            && self.is_word(index + 3, "ZONE")
        {
            index + 3
        } else {
            index
        };
        self.report_span(rule_name, message_id, index, end);
    }

    fn timestamp_has_time_zone(&self, index: usize) -> bool {
        self.is_word(index, "timestamp")
            && self.is_word(index + 1, "WITH")
            && self.is_word(index + 2, "TIME")
            && self.is_word(index + 3, "ZONE")
    }

    fn previous_is(&self, index: usize, word: &str) -> bool {
        index > 0 && self.tokens[index - 1].is_word(word)
    }

    fn statement_contains_word(&self, start: usize, end: usize, word: &str) -> bool {
        (start..end.min(self.tokens.len())).any(|index| self.is_word(index, word))
    }

    fn statement_contains_sequence(&self, start: usize, end: usize, words: &[&str]) -> bool {
        if words.is_empty() {
            return true;
        }
        let mut matched = 0;
        for index in start..end.min(self.tokens.len()) {
            if self.is_word(index, words[matched]) {
                matched += 1;
                if matched == words.len() {
                    return true;
                }
            }
        }
        false
    }

    fn report_statement(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        start: usize,
        end: usize,
    ) {
        if start >= self.tokens.len() {
            return;
        }
        let end = end
            .saturating_sub(1)
            .min(self.tokens.len().saturating_sub(1));
        self.report_span(rule_name, message_id, start, end);
    }

    fn report_span(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        start: usize,
        end: usize,
    ) {
        if let (Some(start_token), Some(end_token)) = (self.token(start), self.token(end)) {
            self.report(
                rule_name,
                message_id,
                Span::new(start_token.span.start, end_token.span.end),
            );
        }
    }

    fn report(&mut self, rule_name: &'static str, message_id: &'static str, span: Span) {
        self.report_with_data(rule_name, message_id, span, DiagnosticData::default());
    }

    fn report_with_data(
        &mut self,
        rule_name: &'static str,
        message_id: &'static str,
        span: Span,
        data: DiagnosticData,
    ) {
        self.diagnostics.push(Diagnostic {
            rule_name,
            message_id,
            data,
            loc: self.line_index.loc_for_span(self.source_text, span),
        });
    }

    fn is_word(&self, index: usize, word: &str) -> bool {
        self.token(index).is_some_and(|token| token.is_word(word))
    }

    fn token(&self, index: usize) -> Option<&Token> {
        self.tokens.get(index)
    }
}

impl Token {
    fn is_word(&self, word: &str) -> bool {
        self.kind == TokenKind::Word && self.text.eq_ignore_ascii_case(word)
    }

    fn is_symbol(&self, symbol: &str) -> bool {
        self.kind == TokenKind::Symbol && self.text == symbol
    }
}

fn lex_sql(source_text: &str) -> SmallVec<[Token; 256]> {
    let bytes = source_text.as_bytes();
    let mut tokens = SmallVec::new();
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if byte == b'-' && bytes.get(index + 1) == Some(&b'-') {
            index = skip_line_comment(bytes, index + 2);
            continue;
        }
        if byte == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index = skip_block_comment(bytes, index + 2);
            continue;
        }
        if byte == b'\'' {
            let end = skip_single_quoted(bytes, index + 1);
            push_token(&mut tokens, TokenKind::String, source_text, index, end);
            index = end;
            continue;
        }
        if byte == b'"' {
            let end = skip_double_quoted(bytes, index + 1);
            push_token(
                &mut tokens,
                TokenKind::QuotedIdentifier,
                source_text,
                index,
                end,
            );
            index = end;
            continue;
        }
        if byte == b'$'
            && let Some(end) = skip_dollar_quoted(bytes, index)
        {
            push_token(&mut tokens, TokenKind::String, source_text, index, end);
            index = end;
            continue;
        }
        if is_word_start(byte) {
            let start = index;
            index += 1;
            while index < bytes.len() && is_word_continue(bytes[index]) {
                index += 1;
            }
            push_token(&mut tokens, TokenKind::Word, source_text, start, index);
            continue;
        }
        if byte.is_ascii_digit() {
            let start = index;
            index += 1;
            while index < bytes.len()
                && (bytes[index].is_ascii_digit() || matches!(bytes[index], b'.' | b'_'))
            {
                index += 1;
            }
            push_token(&mut tokens, TokenKind::Number, source_text, start, index);
            continue;
        }
        let start = index;
        if matches!(
            bytes.get(index..index + 2),
            Some([b':', b':'] | [b'!', b'='] | [b'<', b'>'])
        ) {
            index += 2;
        } else {
            index += 1;
        }
        push_token(&mut tokens, TokenKind::Symbol, source_text, start, index);
    }
    tokens
}

fn push_token(
    tokens: &mut SmallVec<[Token; 256]>,
    kind: TokenKind,
    source_text: &str,
    start: usize,
    end: usize,
) {
    tokens.push(Token {
        kind,
        text: CompactString::from(source_text.get(start..end).map_or("", |text| text)),
        span: Span::new(start as u32, end as u32),
    });
}

fn skip_line_comment(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index] != b'\n' {
        index += 1;
    }
    index
}

fn skip_block_comment(bytes: &[u8], mut index: usize) -> usize {
    while index + 1 < bytes.len() {
        if bytes[index] == b'*' && bytes[index + 1] == b'/' {
            return index + 2;
        }
        index += 1;
    }
    bytes.len()
}

fn skip_single_quoted(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index] == b'\'' {
            if bytes.get(index + 1) == Some(&b'\'') {
                index += 2;
            } else {
                return index + 1;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn skip_double_quoted(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() {
        if bytes[index] == b'"' {
            if bytes.get(index + 1) == Some(&b'"') {
                index += 2;
            } else {
                return index + 1;
            }
        } else {
            index += 1;
        }
    }
    bytes.len()
}

fn skip_dollar_quoted(bytes: &[u8], start: usize) -> Option<usize> {
    let mut tag_end = start + 1;
    while tag_end < bytes.len() && is_word_continue(bytes[tag_end]) {
        tag_end += 1;
    }
    if bytes.get(tag_end) != Some(&b'$') {
        return None;
    }
    let tag = &bytes[start..=tag_end];
    let mut index = tag_end + 1;
    while index + tag.len() <= bytes.len() {
        if &bytes[index..index + tag.len()] == tag {
            return Some(index + tag.len());
        }
        index += 1;
    }
    Some(bytes.len())
}

fn is_word_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_word_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'$')
}

#[cfg(test)]
mod tests {
    use super::{ScanOptions, scan_postgresql};

    fn ids(source_text: &str) -> oxlint_plugins_carton::SmallVec<[&'static str; 32]> {
        scan_postgresql(source_text, "fixture.sql", &ScanOptions::default())
            .into_iter()
            .map(|diagnostic| diagnostic.message_id)
            .collect()
    }

    #[test]
    fn scans_core_sql_rules() {
        let diagnostics =
            ids("DROP DATABASE archive;\nSELECT * FROM users;\nUPDATE users SET active = false;\n");
        assert!(diagnostics.contains(&"noDropDatabase"));
        assert!(diagnostics.contains(&"noSelectStar"));
        assert!(diagnostics.contains(&"missingWhere"));
    }

    #[test]
    fn skips_comments_and_strings() {
        let diagnostics = ids("-- DROP DATABASE nope\nSELECT 'DROP DATABASE nope';\n");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn detects_style_and_type_rules() {
        let diagnostics = ids(
            "CREATE TABLE t (id serial, payload json, name varchar(255), at timestamp, price money);\nSELECT now() != NULL;\n",
        );
        assert!(diagnostics.contains(&"preferIdentity"));
        assert!(diagnostics.contains(&"preferJsonb"));
        assert!(diagnostics.contains(&"preferText"));
        assert!(diagnostics.contains(&"preferTimestamptz"));
        assert!(diagnostics.contains(&"noMoney"));
        assert!(diagnostics.contains(&"useIsNull"));
        assert!(diagnostics.contains(&"preferAngle"));
        assert!(diagnostics.contains(&"preferCurrentTimestamp"));
    }
}
