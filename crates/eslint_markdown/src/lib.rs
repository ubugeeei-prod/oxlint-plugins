#![doc = "Rust implementation of @eslint/markdown rule logic."]

use markdown::mdast;
use oxlint_plugins_carton::{CompactString, FastHashMap, FastHashSet, SmallVec};
use regex::Regex;

// Rules whose logic is driven by the markdown-rs mdast tree (mirroring upstream's
// mdast-based rules) rather than the regex `MarkdownFacts`. The tree is parsed
// once per scan only when one of these rules is active.
const MDAST_RULES: &[&str] = &[
    "no-bare-urls",
    "no-duplicate-headings",
    "no-empty-definitions",
    "no-missing-atx-heading-space",
    "no-missing-label-refs",
    "no-space-in-emphasis",
    "no-unused-definitions",
    "require-alt-text",
];

// Parse the source into an mdast tree with the same constructs upstream enables
// for `markdown/gfm` (GFM tables, autolink literals, footnotes, strikethrough).
fn parse_mdast(source_text: &str, math: bool) -> Option<mdast::Node> {
    let mut options = markdown::ParseOptions::gfm();
    if math {
        options.constructs.math_flow = true;
        options.constructs.math_text = true;
    }
    markdown::to_mdast(source_text, &options).ok()
}

// Depth-first visit of every node in the tree.
fn visit_mdast<'a>(node: &'a mdast::Node, visit: &mut impl FnMut(&'a mdast::Node)) {
    visit(node);
    if let Some(children) = node.children() {
        for child in children {
            visit_mdast(child, visit);
        }
    }
}

// The byte span of a node, taken from its mdast source position. Byte offsets are
// converted to 1-indexed-line / 0-indexed-UTF-16-column locations by `LineIndex`,
// keeping every rule on one column convention.
fn node_span(node: &mdast::Node) -> Option<ByteSpan> {
    node.position().map(|position| ByteSpan {
        start: position.start.offset,
        end: position.end.offset,
    })
}

pub const RULE_NAMES: [&str; 21] = [
    "fenced-code-language",
    "fenced-code-meta",
    "heading-increment",
    "no-bare-urls",
    "no-duplicate-definitions",
    "no-duplicate-headings",
    "no-empty-definitions",
    "no-empty-images",
    "no-empty-links",
    "no-html",
    "no-invalid-label-refs",
    "no-missing-atx-heading-space",
    "no-missing-label-refs",
    "no-missing-link-fragments",
    "no-multiple-h1",
    "no-reference-like-urls",
    "no-reversed-media-syntax",
    "no-space-in-emphasis",
    "no-unused-definitions",
    "require-alt-text",
    "table-column-count",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScanOptions {
    pub rule_names: SmallVec<[CompactString; 21]>,
    pub required_code_languages: SmallVec<[CompactString; 8]>,
    pub fenced_code_meta_mode: MetaMode,
    pub frontmatter_title: Option<CompactString>,
    pub check_closed_headings: bool,
    pub check_strikethrough: bool,
    pub allowed_html: SmallVec<[CompactString; 8]>,
    pub allowed_html_ignore_case: bool,
    pub allow_labels: SmallVec<[CompactString; 8]>,
    pub allow_definitions: SmallVec<[CompactString; 8]>,
    pub allow_footnote_definitions: SmallVec<[CompactString; 8]>,
    pub check_footnote_definitions: bool,
    pub check_duplicate_headings_siblings_only: bool,
    pub ignore_fragment_case: bool,
    pub allow_fragment_pattern: Option<CompactString>,
    pub check_missing_table_cells: bool,
    /// Whether `$...$` / `$$...$$` math is parsed (upstream `languageOptions.math`).
    pub math: bool,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            rule_names: SmallVec::new(),
            required_code_languages: SmallVec::new(),
            fenced_code_meta_mode: MetaMode::Always,
            frontmatter_title: None,
            check_closed_headings: false,
            check_strikethrough: false,
            allowed_html: SmallVec::new(),
            allowed_html_ignore_case: false,
            allow_labels: SmallVec::new(),
            allow_definitions: {
                let mut values = SmallVec::new();
                values.push(CompactString::from("//"));
                values
            },
            allow_footnote_definitions: SmallVec::new(),
            check_footnote_definitions: true,
            check_duplicate_headings_siblings_only: false,
            ignore_fragment_case: true,
            allow_fragment_pattern: None,
            check_missing_table_cells: false,
            math: false,
        }
    }
}

impl ScanOptions {
    fn is_enabled(&self, rule_name: &str) -> bool {
        self.rule_names.is_empty() || self.rule_names.iter().any(|name| name == rule_name)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MetaMode {
    #[default]
    Always,
    Never,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DiagnosticData {
    pub lang: Option<CompactString>,
    pub name: Option<CompactString>,
    pub identifier: Option<CompactString>,
    pub label: Option<CompactString>,
    pub first_line: Option<u32>,
    pub first_label: Option<CompactString>,
    pub from_level: Option<u32>,
    pub to_level: Option<u32>,
    pub position: Option<CompactString>,
    pub text: Option<CompactString>,
    pub link_type: Option<CompactString>,
    pub prefix: Option<CompactString>,
    pub fragment: Option<CompactString>,
    pub expected_cells: Option<u32>,
    pub actual_cells: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticLoc {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagnosticFix {
    pub start: u32,
    pub end: u32,
    pub replacement: CompactString,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub rule_name: &'static str,
    pub message_id: &'static str,
    pub data: DiagnosticData,
    pub loc: DiagnosticLoc,
    pub fix: Option<DiagnosticFix>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ByteSpan {
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LineFact<'a> {
    number: u32,
    start: usize,
    end: usize,
    text: &'a str,
    in_fence: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FenceFact {
    marker_span: ByteSpan,
    info_span: ByteSpan,
    lang: Option<CompactString>,
    meta: Option<CompactString>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HeadingFact {
    depth: u8,
    text: CompactString,
    span: ByteSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionFact {
    identifier: CompactString,
    label: CompactString,
    url: CompactString,
    span: ByteSpan,
    is_footnote: bool,
    line: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LinkFact {
    is_image: bool,
    label: CompactString,
    url: CompactString,
    span: ByteSpan,
    url_span: ByteSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LabelRefFact {
    label: CompactString,
    span: ByteSpan,
    is_footnote: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HtmlTagFact {
    name: CompactString,
    span: ByteSpan,
    raw: CompactString,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct MarkdownFacts<'a> {
    lines: SmallVec<[LineFact<'a>; 64]>,
    fences: SmallVec<[FenceFact; 8]>,
    headings: SmallVec<[HeadingFact; 16]>,
    definitions: SmallVec<[DefinitionFact; 16]>,
    links: SmallVec<[LinkFact; 32]>,
    label_refs: SmallVec<[LabelRefFact; 32]>,
    html_tags: SmallVec<[HtmlTagFact; 16]>,
    frontmatter_has_title: bool,
}

pub fn implemented_eslint_markdown_rule_names() -> &'static [&'static str] {
    &RULE_NAMES
}

pub fn scan_eslint_markdown(
    source_text: &str,
    options: &ScanOptions,
) -> SmallVec<[Diagnostic; 32]> {
    let line_index = LineIndex::new(source_text);
    let facts = collect_facts(source_text, options);
    let mdast = if MDAST_RULES.iter().any(|rule| options.is_enabled(rule)) {
        parse_mdast(source_text, options.math)
    } else {
        None
    };
    let mut diagnostics = SmallVec::new();

    if options.is_enabled("fenced-code-language") {
        scan_fenced_code_language(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("fenced-code-meta") {
        scan_fenced_code_meta(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("heading-increment") {
        scan_heading_increment(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-bare-urls")
        && let Some(tree) = &mdast
    {
        scan_bare_urls(source_text, &line_index, tree, &mut diagnostics);
    }
    if options.is_enabled("no-duplicate-definitions") {
        scan_duplicate_definitions(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-duplicate-headings")
        && let Some(tree) = &mdast
    {
        scan_duplicate_headings(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("no-empty-definitions")
        && let Some(tree) = &mdast
    {
        scan_empty_definitions(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("no-empty-images") {
        scan_empty_media(source_text, &line_index, &facts, true, &mut diagnostics);
    }
    if options.is_enabled("no-empty-links") {
        scan_empty_media(source_text, &line_index, &facts, false, &mut diagnostics);
    }
    if options.is_enabled("no-html") {
        scan_no_html(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-invalid-label-refs") {
        scan_invalid_label_refs(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-missing-atx-heading-space")
        && let Some(tree) = &mdast
    {
        scan_missing_atx_heading_space(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("no-missing-label-refs")
        && let Some(tree) = &mdast
    {
        scan_missing_label_refs(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("no-missing-link-fragments") {
        scan_missing_link_fragments(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-multiple-h1") {
        scan_multiple_h1(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-reference-like-urls") {
        scan_reference_like_urls(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-reversed-media-syntax") {
        scan_reversed_media(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-space-in-emphasis")
        && let Some(tree) = &mdast
    {
        scan_space_in_emphasis(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("no-unused-definitions")
        && let Some(tree) = &mdast
    {
        scan_unused_definitions(source_text, &line_index, tree, options, &mut diagnostics);
    }
    if options.is_enabled("require-alt-text")
        && let Some(tree) = &mdast
    {
        scan_require_alt_text(source_text, &line_index, tree, &mut diagnostics);
    }
    if options.is_enabled("table-column-count") {
        scan_table_column_count(source_text, &line_index, &facts, options, &mut diagnostics);
    }

    diagnostics
}

fn collect_facts<'a>(source_text: &'a str, options: &ScanOptions) -> MarkdownFacts<'a> {
    let mut facts = MarkdownFacts::default();
    collect_lines(source_text, &mut facts);
    collect_frontmatter(
        source_text,
        &mut facts,
        options.frontmatter_title.as_deref(),
    );
    collect_fences(source_text, &mut facts);
    collect_headings(&mut facts);
    collect_definitions(&mut facts);
    collect_inline_links(source_text, &mut facts);
    collect_html_tags(source_text, &mut facts);
    facts
}

fn collect_lines<'a>(source_text: &'a str, facts: &mut MarkdownFacts<'a>) {
    let bytes = source_text.as_bytes();
    let mut start = 0usize;
    let mut number = 1u32;
    while start <= source_text.len() {
        let mut end = start;
        while end < source_text.len() && bytes[end] != b'\n' {
            end += 1;
        }
        let text_end = if end > start && bytes[end.saturating_sub(1)] == b'\r' {
            end - 1
        } else {
            end
        };
        let text = source_text.get(start..text_end).unwrap_or("");
        facts.lines.push(LineFact {
            number,
            start,
            end: text_end,
            text,
            in_fence: false,
        });
        if end == source_text.len() {
            break;
        }
        start = end + 1;
        number += 1;
    }
}

fn collect_frontmatter(
    source_text: &str,
    facts: &mut MarkdownFacts<'_>,
    frontmatter_title: Option<&str>,
) {
    if frontmatter_title == Some("") {
        return;
    }
    let Some(first) = facts.lines.first() else {
        return;
    };
    let marker = first.text.trim();
    if !matches!(marker, "---" | "+++" | "{") {
        return;
    }
    let title_pattern = frontmatter_title.and_then(|pattern| Regex::new(pattern).ok());
    for line in facts.lines.iter().skip(1).take(32) {
        let text = line.text.trim();
        let matched_title = title_pattern
            .as_ref()
            .is_some_and(|pattern| pattern.is_match(text))
            || (title_pattern.is_none()
                && (text.starts_with("title")
                    || text.starts_with("\"title\"")
                    || text.starts_with("'title'")));
        if matched_title {
            facts.frontmatter_has_title = true;
            return;
        }
        if text == marker || (marker == "{" && text == "}") {
            return;
        }
    }
    if title_pattern
        .as_ref()
        .is_some_and(|pattern| pattern.is_match(source_text))
        || (title_pattern.is_none() && source_text.starts_with("title:"))
    {
        facts.frontmatter_has_title = true;
    }
}

fn collect_fences(source_text: &str, facts: &mut MarkdownFacts<'_>) {
    let mut open_marker: Option<(u8, usize)> = None;
    for line in &mut facts.lines {
        let (indent, trimmed) = split_indent(line.text);
        if indent > 3 {
            line.in_fence = open_marker.is_some();
            continue;
        }
        let marker_byte = trimmed.as_bytes().first().copied();
        let marker_len = trimmed
            .bytes()
            .take_while(|byte| Some(*byte) == marker_byte)
            .count();
        let is_fence = matches!(marker_byte, Some(b'`' | b'~')) && marker_len >= 3;
        if let Some((open_byte, open_len)) = open_marker {
            line.in_fence = true;
            if is_fence && marker_byte == Some(open_byte) && marker_len >= open_len {
                open_marker = None;
            }
            continue;
        }
        if !is_fence {
            continue;
        }

        line.in_fence = true;
        let marker_start = line.start + indent;
        let marker_end = marker_start + marker_len;
        let info_start = marker_end;
        let info = source_text.get(info_start..line.end).unwrap_or("").trim();
        let (lang, meta) = split_fence_info(info);
        facts.fences.push(FenceFact {
            marker_span: ByteSpan {
                start: marker_start,
                end: marker_end,
            },
            info_span: ByteSpan {
                start: info_start,
                end: line.end,
            },
            lang,
            meta,
        });
        open_marker = marker_byte.map(|byte| (byte, marker_len));
    }
}

fn collect_headings(facts: &mut MarkdownFacts<'_>) {
    for line in &facts.lines {
        if line.in_fence {
            continue;
        }
        if let Some((depth, text)) = parse_atx_heading(line.text) {
            facts.headings.push(HeadingFact {
                depth,
                text: CompactString::from(text),
                span: ByteSpan {
                    start: line.start,
                    end: line.end,
                },
            });
        }
    }

    for index in 1..facts.lines.len() {
        let line = &facts.lines[index];
        if line.in_fence {
            continue;
        }
        let trimmed = line.text.trim();
        let depth = if trimmed.chars().all(|ch| ch == '=') && trimmed.len() >= 3 {
            Some(1)
        } else if trimmed.chars().all(|ch| ch == '-') && trimmed.len() >= 3 {
            Some(2)
        } else {
            None
        };
        let Some(depth) = depth else {
            continue;
        };
        let prev = &facts.lines[index - 1];
        if prev.text.trim().is_empty() || prev.in_fence {
            continue;
        }
        facts.headings.push(HeadingFact {
            depth,
            text: CompactString::from(prev.text.trim()),
            span: ByteSpan {
                start: prev.start,
                end: line.end,
            },
        });
    }
}

fn collect_definitions(facts: &mut MarkdownFacts<'_>) {
    for line in &facts.lines {
        if line.in_fence {
            continue;
        }
        // Lines indented >= 4 columns are indented code blocks, not definitions.
        if indent_columns(line.text) >= 4 {
            continue;
        }
        let trimmed = line.text.trim_start();
        let Some(rest) = trimmed.strip_prefix('[') else {
            continue;
        };
        let is_footnote = rest.starts_with('^');
        let label_start = usize::from(is_footnote);
        let Some(close) = rest.find("]:") else {
            continue;
        };
        let raw_label = &rest[label_start..close];
        let url = rest[close + 2..].trim();
        let label = CompactString::from(raw_label.trim());
        let identifier = normalize_identifier(raw_label);
        facts.definitions.push(DefinitionFact {
            identifier,
            label,
            url: CompactString::from(trim_definition_url(url)),
            span: ByteSpan {
                start: line.start + (line.text.len() - trimmed.len()),
                end: line.end,
            },
            is_footnote,
            line: line.number,
        });
    }
}

fn collect_inline_links(source_text: &str, facts: &mut MarkdownFacts<'_>) {
    let Ok(inline_link_re) = Regex::new(r"!?\[[^\]\n]*\]\([^\)\n]*\)") else {
        return;
    };
    let Ok(label_ref_re) = Regex::new(r"!?\[[^\]\n]*\](?:\[[^\]\n]*\])?") else {
        return;
    };

    for line in &facts.lines {
        if line.in_fence || is_definition_line(line.text) {
            continue;
        }
        for mat in inline_link_re.find_iter(line.text) {
            let raw = mat.as_str();
            if let Some(link) = parse_inline_link(raw, line.start + mat.start()) {
                facts.links.push(link);
            }
        }
        for mat in label_ref_re.find_iter(line.text) {
            let absolute_start = line.start + mat.start();
            let absolute_end = line.start + mat.end();
            if source_text.get(absolute_end..absolute_end + 1) == Some("(") {
                continue;
            }
            if let Some(label_ref) = parse_label_ref(mat.as_str(), absolute_start, absolute_end) {
                facts.label_refs.push(label_ref);
            }
        }
    }
}

fn collect_html_tags(source_text: &str, facts: &mut MarkdownFacts<'_>) {
    // Mirror upstream's `htmlTagPattern`: an attribute value may contain `>`
    // when wrapped in quotes, so the tag does not end at the first raw `>`.
    let Ok(tag_re) =
        Regex::new(r#"(?i)<[a-z0-9]+(?:-[a-z0-9]+)*(?:\s(?:[^>"']|"[^"]*"|'[^']*')*)?/?>"#)
    else {
        return;
    };
    // Upstream strips HTML comments before scanning for tags; blank out comment
    // interiors first (preserving byte length and newlines so offsets stay valid).
    let stripped = strip_html_comments(source_text);
    for mat in tag_re.find_iter(&stripped) {
        if in_fenced_range(facts, mat.start()) {
            continue;
        }
        let raw = source_text.get(mat.start()..mat.end()).unwrap_or("");
        let name = html_tag_name(raw);
        if name.is_empty() {
            continue;
        }
        facts.html_tags.push(HtmlTagFact {
            name,
            span: ByteSpan {
                start: mat.start(),
                end: mat.end(),
            },
            raw: CompactString::from(raw),
        });
    }
}

// Replace the interior of every `<!-- ... -->` comment with spaces, keeping the
// byte length and any newlines so byte offsets into the original text stay valid
// (mirrors upstream `stripHtmlComments`, which replaces each non-newline unit).
fn strip_html_comments(source_text: &str) -> CompactString {
    let Ok(comment_re) = Regex::new(r"(?s)<!--.*?-->") else {
        return CompactString::from(source_text);
    };
    let replaced = comment_re.replace_all(source_text, |caps: &regex::Captures| {
        caps[0]
            .bytes()
            .map(|byte| match byte {
                b'\n' => '\n',
                b'\r' => '\r',
                _ => ' ',
            })
            .collect::<CompactString>()
    });
    CompactString::from(replaced.as_ref())
}

// Span from the opening fence start through the end of the language token,
// matching upstream's `start: node.position.start` + `column + langIndex +
// lang.length` (where `langIndex` is the offset of the language within the
// node's text, i.e. on the opening fence line).
fn fence_lang_span(source_text: &str, fence: &FenceFact, lang: &str) -> ByteSpan {
    let fence_start = fence.marker_span.start;
    let fence_line = source_text
        .get(fence_start..fence.info_span.end)
        .unwrap_or("");
    let lang_offset = fence_line.find(lang).unwrap_or(0);
    ByteSpan {
        start: fence_start,
        end: fence_start + lang_offset + lang.len(),
    }
}

// Span covering the metadata token on the opening fence line, matching upstream's
// `start.column + metaIndex` .. `+ meta.trimEnd().length` (where `metaIndex` is
// `fenceLineText.lastIndexOf(node.meta)`).
fn fence_meta_span(source_text: &str, fence: &FenceFact, meta: &str) -> ByteSpan {
    let fence_start = fence.marker_span.start;
    let fence_line = source_text
        .get(fence_start..fence.info_span.end)
        .unwrap_or("");
    let meta_offset = fence_line.rfind(meta).unwrap_or(0);
    let start = fence_start + meta_offset;
    ByteSpan {
        start,
        end: start + meta.trim_end().len(),
    }
}

fn scan_fenced_code_language(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for fence in &facts.fences {
        let Some(lang) = &fence.lang else {
            diagnostics.push(Diagnostic {
                rule_name: "fenced-code-language",
                message_id: "missingLanguage",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, fence.marker_span),
                fix: None,
            });
            continue;
        };
        if !options.required_code_languages.is_empty()
            && !options
                .required_code_languages
                .iter()
                .any(|value| value == lang)
        {
            diagnostics.push(Diagnostic {
                rule_name: "fenced-code-language",
                message_id: "disallowedLanguage",
                data: DiagnosticData {
                    lang: Some(lang.clone()),
                    ..DiagnosticData::default()
                },
                loc: line_index
                    .loc_for_span(source_text, fence_lang_span(source_text, fence, lang)),
                fix: None,
            });
        }
    }
}

fn scan_fenced_code_meta(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for fence in &facts.fences {
        match options.fenced_code_meta_mode {
            MetaMode::Always => {
                let Some(lang) = &fence.lang else {
                    continue;
                };
                if fence
                    .meta
                    .as_ref()
                    .is_none_or(|meta| meta.trim().is_empty())
                {
                    diagnostics.push(Diagnostic {
                        rule_name: "fenced-code-meta",
                        message_id: "missingMetadata",
                        data: DiagnosticData::default(),
                        loc: line_index
                            .loc_for_span(source_text, fence_lang_span(source_text, fence, lang)),
                        fix: None,
                    });
                }
            }
            MetaMode::Never => {
                if let Some(meta) = fence.meta.as_ref().filter(|meta| !meta.trim().is_empty()) {
                    diagnostics.push(Diagnostic {
                        rule_name: "fenced-code-meta",
                        message_id: "disallowedMetadata",
                        data: DiagnosticData::default(),
                        loc: line_index
                            .loc_for_span(source_text, fence_meta_span(source_text, fence, meta)),
                        fix: None,
                    });
                }
            }
        }
    }
}

fn scan_heading_increment(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let mut last_depth = if facts.frontmatter_has_title { 1 } else { 0 };
    let mut headings = facts.headings.iter().collect::<SmallVec<[_; 16]>>();
    headings.sort_by_key(|heading| heading.span.start);
    for heading in headings {
        let depth = u32::from(heading.depth);
        if last_depth > 0 && depth > last_depth + 1 {
            diagnostics.push(Diagnostic {
                rule_name: "heading-increment",
                message_id: "skippedHeading",
                data: DiagnosticData {
                    from_level: Some(last_depth),
                    to_level: Some(depth),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, heading.span),
                fix: None,
            });
        }
        last_depth = depth;
    }
}

fn scan_bare_urls(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // Collect the Link nodes (in document order) that are NOT enclosed in a
    // matched HTML tag pair within a heading / paragraph / table cell.
    let mut links: SmallVec<[&mdast::Node; 16]> = SmallVec::new();
    collect_bare_url_links(tree, &mut links);

    for link in &links {
        let mdast::Node::Link(link_data) = link else {
            continue;
        };
        let Some(span) = node_span(link) else {
            continue;
        };
        let text = source_text.get(span.start..span.end).unwrap_or("");
        // A bare URL is a GFM autolink literal whose source text equals its URL
        // (optionally with an implied `http://` / `mailto:` scheme).
        let url = link_data.url.as_str();
        let is_bare = url == text
            || url.strip_prefix("http://") == Some(text)
            || url.strip_prefix("mailto:") == Some(text);
        if !is_bare {
            continue;
        }
        let mut replacement = CompactString::from("<");
        replacement.push_str(text);
        replacement.push('>');
        diagnostics.push(Diagnostic {
            rule_name: "no-bare-urls",
            message_id: "bareUrl",
            data: DiagnosticData::default(),
            loc: line_index.loc_for_span(source_text, span),
            fix: Some(DiagnosticFix {
                start: utf16_offset(source_text, span.start),
                end: utf16_offset(source_text, span.end),
                replacement,
            }),
        });
    }
}

fn collect_bare_url_links<'a>(node: &'a mdast::Node, links: &mut SmallVec<[&'a mdast::Node; 16]>) {
    if matches!(
        node,
        mdast::Node::Heading(_) | mdast::Node::Paragraph(_) | mdast::Node::TableCell(_)
    ) {
        let mut tentative: SmallVec<[&mdast::Node; 8]> = SmallVec::new();
        let mut open_tag: Option<CompactString> = None;
        collect_container_links(node, &mut tentative, &mut open_tag, links);
        // Links after an unclosed opening tag are still reported.
        links.extend(tentative);
    } else if let Some(children) = node.children() {
        for child in children {
            collect_bare_url_links(child, links);
        }
    }
}

// Walk a container's descendants in document order, tracking HTML tag pairing:
// a link between a matched `<tag>`...`</tag>` pair is dropped, every other link
// is reported.
fn collect_container_links<'a>(
    node: &'a mdast::Node,
    tentative: &mut SmallVec<[&'a mdast::Node; 8]>,
    open_tag: &mut Option<CompactString>,
    links: &mut SmallVec<[&'a mdast::Node; 16]>,
) {
    let Some(children) = node.children() else {
        return;
    };
    for child in children {
        match child {
            mdast::Node::Html(html) => {
                if let Some((name, is_closing)) = parse_html_tag(&html.value) {
                    if !is_closing && open_tag.is_none() {
                        *open_tag = Some(name);
                    } else if is_closing && open_tag.as_deref() == Some(name.as_str()) {
                        tentative.clear();
                        *open_tag = None;
                    }
                }
            }
            mdast::Node::Link(_) => {
                if open_tag.is_some() {
                    tentative.push(child);
                } else {
                    links.push(child);
                }
            }
            _ => {}
        }
        // Don't descend into a link's own children: links can't nest, but
        // markdown-rs still parses an autolink literal inside link text, which
        // upstream (micromark) does not.
        if !matches!(child, mdast::Node::Link(_)) {
            collect_container_links(child, tentative, open_tag, links);
        }
    }
}

// Parse an HTML tag's name and whether it is a closing tag, matching upstream's
// `/^<(?<tagName>[^!>][^/\s>]*)/` + lowercasing.
fn parse_html_tag(tag_text: &str) -> Option<(CompactString, bool)> {
    let rest = tag_text.strip_prefix('<')?;
    let first = rest.chars().next()?;
    if first == '!' || first == '>' {
        return None;
    }
    let mut name = CompactString::new("");
    name.push(first);
    for ch in rest.chars().skip(1) {
        if ch == '/' || ch.is_whitespace() || ch == '>' {
            break;
        }
        name.push(ch);
    }
    let name = lower(name.as_str());
    match name.strip_prefix('/') {
        Some(rest) => Some((CompactString::from(rest), true)),
        None => Some((name, false)),
    }
}

fn scan_duplicate_definitions(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let mut definitions = FastHashMap::<CompactString, &DefinitionFact>::default();
    let mut footnotes = FastHashMap::<CompactString, &DefinitionFact>::default();
    for definition in &facts.definitions {
        if definition.is_footnote {
            if !options.check_footnote_definitions
                || is_allowed(&options.allow_footnote_definitions, &definition.identifier)
            {
                continue;
            }
            if let Some(first) = footnotes.get(&definition.identifier) {
                diagnostics.push(definition_diagnostic(
                    "no-duplicate-definitions",
                    "duplicateFootnoteDefinition",
                    source_text,
                    line_index,
                    definition,
                    Some(first),
                ));
            } else {
                footnotes.insert(definition.identifier.clone(), definition);
            }
        } else {
            if is_allowed(&options.allow_definitions, &definition.identifier) {
                continue;
            }
            if let Some(first) = definitions.get(&definition.identifier) {
                diagnostics.push(definition_diagnostic(
                    "no-duplicate-definitions",
                    "duplicateDefinition",
                    source_text,
                    line_index,
                    definition,
                    Some(first),
                ));
            } else {
                definitions.insert(definition.identifier.clone(), definition);
            }
        }
    }
}

fn scan_duplicate_headings(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // (depth, structural-sequence key, display text, span), in document order.
    let mut headings: SmallVec<[(u8, CompactString, CompactString, ByteSpan); 16]> =
        SmallVec::new();
    visit_mdast(tree, &mut |node| {
        if let mdast::Node::Heading(heading) = node
            && let Some(span) = node_span(node)
        {
            let (sequence, text) = heading_sequence_and_text(node);
            headings.push((heading.depth, sequence, text, span));
        }
    });

    if options.check_duplicate_headings_siblings_only {
        // Per-level sets: descending to a shallower level clears the deeper ones,
        // so only true siblings collide.
        let mut by_level: [FastHashSet<CompactString>; 7] =
            core::array::from_fn(|_| FastHashSet::default());
        let mut last_level = 1u8;
        for (depth, sequence, text, span) in &headings {
            let level = (*depth).clamp(1, 6);
            if level < last_level {
                for set in by_level
                    .iter_mut()
                    .take(usize::from(last_level) + 1)
                    .skip(usize::from(level) + 1)
                {
                    set.clear();
                }
            }
            last_level = level;
            if !by_level[usize::from(level)].insert(sequence.clone()) {
                push_duplicate_heading(source_text, line_index, text, *span, diagnostics);
            }
        }
    } else {
        let mut seen = FastHashSet::<CompactString>::default();
        for (_, sequence, text, span) in &headings {
            if !seen.insert(sequence.clone()) {
                push_duplicate_heading(source_text, line_index, text, *span, diagnostics);
            }
        }
    }
}

fn push_duplicate_heading(
    source_text: &str,
    line_index: &LineIndex,
    text: &CompactString,
    span: ByteSpan,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    diagnostics.push(Diagnostic {
        rule_name: "no-duplicate-headings",
        message_id: "duplicateHeading",
        data: DiagnosticData {
            text: Some(text.clone()),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, span),
        fix: None,
    });
}

// Build upstream's `headingChildrenSequence` (a structural key of the heading's
// phrasing children) and `headingText` (the concatenated values, excluding HTML
// node values), walking the heading's descendants in document order.
fn heading_sequence_and_text(heading: &mdast::Node) -> (CompactString, CompactString) {
    let mut sequence = CompactString::new("");
    let mut text = CompactString::new("");
    if let Some(children) = heading.children() {
        for child in children {
            visit_mdast(child, &mut |node| {
                let type_name = phrasing_type_name(node);
                match phrasing_value(node) {
                    Some(value) => {
                        sequence.push('[');
                        sequence.push_str(type_name);
                        sequence.push(',');
                        sequence.push_str(value);
                        sequence.push(']');
                        if type_name != "html" {
                            text.push_str(value);
                        }
                    }
                    None => {
                        sequence.push('[');
                        sequence.push_str(type_name);
                        sequence.push(']');
                    }
                }
            });
        }
    }
    (sequence, text)
}

// The mdast type name of a phrasing node, used only as a stable token in the
// structural sequence key (each variant maps to a distinct string).
fn phrasing_type_name(node: &mdast::Node) -> &'static str {
    match node {
        mdast::Node::Text(_) => "text",
        mdast::Node::Emphasis(_) => "emphasis",
        mdast::Node::Strong(_) => "strong",
        mdast::Node::Delete(_) => "delete",
        mdast::Node::InlineCode(_) => "inlineCode",
        mdast::Node::InlineMath(_) => "inlineMath",
        mdast::Node::Break(_) => "break",
        mdast::Node::Link(_) => "link",
        mdast::Node::Image(_) => "image",
        mdast::Node::LinkReference(_) => "linkReference",
        mdast::Node::ImageReference(_) => "imageReference",
        mdast::Node::FootnoteReference(_) => "footnoteReference",
        mdast::Node::Html(_) => "html",
        _ => "other",
    }
}

// The `value` of a value-bearing phrasing node (text / inline code / inline math
// / raw HTML), if any.
fn phrasing_value(node: &mdast::Node) -> Option<&str> {
    match node {
        mdast::Node::Text(text) => Some(&text.value),
        mdast::Node::InlineCode(code) => Some(&code.value),
        mdast::Node::InlineMath(math) => Some(&math.value),
        mdast::Node::Html(html) => Some(&html.value),
        _ => None,
    }
}

fn scan_empty_definitions(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    visit_mdast(tree, &mut |node| match node {
        // A link definition with no destination (`[x]:`, `[x]: <>`) or a bare
        // fragment (`[x]: #`).
        mdast::Node::Definition(definition) => {
            let label = definition.label.as_deref().unwrap_or("");
            let identifier = normalize_identifier(label);
            if (definition.url.is_empty() || definition.url == "#")
                && !is_allowed(&options.allow_definitions, &identifier)
            {
                push_empty_definition(
                    "emptyDefinition",
                    source_text,
                    line_index,
                    node_span(node),
                    &identifier,
                    label.trim(),
                    diagnostics,
                );
            }
        }
        // A footnote definition with no content, or whose only content is HTML
        // comments.
        mdast::Node::FootnoteDefinition(definition) if options.check_footnote_definitions => {
            let label = definition.label.as_deref().unwrap_or("");
            let identifier = normalize_identifier(label);
            let empty = definition.children.is_empty()
                || definition
                    .children
                    .iter()
                    .all(|child| matches!(child, mdast::Node::Html(html) if is_only_comments(&html.value)));
            if empty && !is_allowed(&options.allow_footnote_definitions, &identifier) {
                push_empty_definition(
                    "emptyFootnoteDefinition",
                    source_text,
                    line_index,
                    node_span(node),
                    &identifier,
                    label,
                    diagnostics,
                );
            }
        }
        _ => {}
    });
}

#[allow(clippy::too_many_arguments)]
fn push_empty_definition(
    message_id: &'static str,
    source_text: &str,
    line_index: &LineIndex,
    span: Option<ByteSpan>,
    identifier: &str,
    label: &str,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    if let Some(span) = span {
        diagnostics.push(Diagnostic {
            rule_name: "no-empty-definitions",
            message_id,
            data: DiagnosticData {
                identifier: Some(CompactString::from(identifier)),
                label: Some(CompactString::from(label)),
                ..DiagnosticData::default()
            },
            loc: line_index.loc_for_span(source_text, span),
            fix: None,
        });
    }
}

// True when the string contains only HTML comments (plus whitespace), matching
// upstream `isOnlyComments`.
fn is_only_comments(value: &str) -> bool {
    strip_html_comments(value).trim().is_empty()
}

fn scan_empty_media(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    images: bool,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for link in &facts.links {
        if link.is_image != images {
            continue;
        }
        if link.url.trim().is_empty() || link.url.trim() == "#" {
            diagnostics.push(Diagnostic {
                rule_name: if images {
                    "no-empty-images"
                } else {
                    "no-empty-links"
                },
                message_id: if images { "emptyImage" } else { "emptyLink" },
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, link.span),
                fix: None,
            });
        }
    }
}

fn scan_no_html(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for tag in &facts.html_tags {
        let normalized = if options.allowed_html_ignore_case {
            lower(tag.name.as_str())
        } else {
            tag.name.clone()
        };
        if options.allowed_html.iter().any(|allowed| {
            if options.allowed_html_ignore_case {
                lower(allowed.as_str()) == normalized
            } else {
                allowed.as_str() == normalized.as_str()
            }
        }) {
            continue;
        }
        // Upstream truncates the reported span at the first line ending inside
        // the matched tag (`firstNewlineIndex`).
        let raw = source_text.get(tag.span.start..tag.span.end).unwrap_or("");
        let end = raw
            .find(['\n', '\r'])
            .map_or(tag.span.end, |idx| tag.span.start + idx);
        diagnostics.push(Diagnostic {
            rule_name: "no-html",
            message_id: "disallowedElement",
            data: DiagnosticData {
                name: Some(tag.name.clone()),
                ..DiagnosticData::default()
            },
            loc: line_index.loc_for_span(
                source_text,
                ByteSpan {
                    start: tag.span.start,
                    end,
                },
            ),
            fix: None,
        });
    }
}

fn scan_invalid_label_refs(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // Mirror upstream: scan for `][label]` (`labelPattern`), and flag it only
    // when the trailing reference is whitespace-only (`illegalShorthandTailPattern`
    // = `/\]\[\s+\]$/`). The reported label is the leading bracket's contents.
    let Ok(label_re) = Regex::new(r"\]\[([^\]]+)\]") else {
        return;
    };
    let Ok(first_re) = Regex::new(r"!?\[([^\]]+)\]") else {
        return;
    };
    let mut search_from = 0;
    while search_from < source_text.len() {
        let Some(mat) = label_re.find_at(source_text, search_from) else {
            break;
        };
        let inner = &mat.as_str()[2..mat.as_str().len() - 1];
        // A blank line (>= 2 newlines in the whitespace) breaks the paragraph, so
        // the `][ ]` would span two text nodes and upstream never matches it.
        let whitespace_only = !inner.is_empty()
            && inner.chars().all(char::is_whitespace)
            && inner.bytes().filter(|byte| *byte == b'\n').count() <= 1;
        if whitespace_only && !in_fenced_range(facts, mat.start()) {
            // The leading `[` of the reference text, searched across the whole
            // document like upstream's `lastIndexOf("[", startOffset)`.
            if let Some(open) = source_text[..mat.start()].rfind('[') {
                let label = first_re
                    .captures(&source_text[open..mat.end()])
                    .and_then(|caps| caps.get(1))
                    .map_or("", |group| group.as_str().trim());
                diagnostics.push(Diagnostic {
                    rule_name: "no-invalid-label-refs",
                    message_id: "invalidLabelRef",
                    data: DiagnosticData {
                        label: Some(CompactString::from(label)),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(
                        source_text,
                        ByteSpan {
                            start: mat.start() + 1,
                            end: mat.end(),
                        },
                    ),
                    fix: None,
                });
            }
        }
        search_from = mat.end();
    }
}

fn scan_missing_atx_heading_space(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    visit_mdast(tree, &mut |node| match node {
        // Heading-like text that is NOT a heading (no space after the hashes)
        // lands in a paragraph; flag the missing space "after" the hashes.
        mdast::Node::Paragraph(_) => {
            if let Some(span) = node_span(node) {
                scan_atx_missing_space_after(source_text, line_index, span, diagnostics);
            }
        }
        // For a real heading, optionally flag a closing `#` sequence with no
        // space "before" it.
        mdast::Node::Heading(_) if options.check_closed_headings => {
            if let Some(span) = node_span(node) {
                scan_atx_missing_space_before(source_text, line_index, span, diagnostics);
            }
        }
        _ => {}
    });
}

// Upstream `leadingAtxHeadingHashPattern`: at the start of any line in the
// paragraph, `#{1,6}` immediately followed by a non-(`#`/space/tab) character.
fn scan_atx_missing_space_after(
    source_text: &str,
    line_index: &LineIndex,
    span: ByteSpan,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let text = source_text.get(span.start..span.end).unwrap_or("");
    let bytes = text.as_bytes();
    let mut line_start = 0;
    loop {
        // markdown-rs keeps a paragraph's leading indentation in the node span,
        // whereas micromark strips it; skip leading whitespace so the hashes are
        // tested at the line's first non-space (matching upstream's `^#{1,6}`).
        let indent = bytes[line_start..]
            .iter()
            .take_while(|byte| matches!(**byte, b' ' | b'\t'))
            .count();
        let content = line_start + indent;
        let hashes = bytes[content..]
            .iter()
            .take_while(|byte| **byte == b'#')
            .count();
        if (1..=6).contains(&hashes) {
            let after = bytes.get(content + hashes).copied();
            if !matches!(after, Some(b'#' | b' ' | b'\t')) {
                let hash_start = span.start + content;
                let hash_end = hash_start + hashes;
                // Upstream end is `endOffset + 1`: one unit (one whole character)
                // past the hashes.
                let end = source_text[hash_end..]
                    .chars()
                    .next()
                    .map_or(hash_end, |ch| hash_end + ch.len_utf8());
                diagnostics.push(Diagnostic {
                    rule_name: "no-missing-atx-heading-space",
                    message_id: "missingSpace",
                    data: DiagnosticData {
                        position: Some(CompactString::from("after")),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(
                        source_text,
                        ByteSpan {
                            start: hash_start,
                            end,
                        },
                    ),
                    fix: Some(DiagnosticFix {
                        start: utf16_offset(source_text, hash_end),
                        end: utf16_offset(source_text, hash_end),
                        replacement: CompactString::from(" "),
                    }),
                });
            }
        }
        match text[line_start..].find(['\n', '\r']) {
            Some(rel) => line_start += rel + 1,
            None => break,
        }
    }
}

// Upstream `trailingAtxHeadingHashPattern`: a trailing `#+` (optionally followed
// by spaces) that is not preceded by a space (so the closing sequence touches
// the content) and is not backslash-escaped.
fn scan_atx_missing_space_before(
    source_text: &str,
    line_index: &LineIndex,
    span: ByteSpan,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let text = source_text.get(span.start..span.end).unwrap_or("");
    let bytes = text.as_bytes();
    let mut end = bytes.len();
    while end > 0 && matches!(bytes[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    let mut hash_start = end;
    while hash_start > 0 && bytes[hash_start - 1] == b'#' {
        hash_start -= 1;
    }
    let mut hashes = end - hash_start;
    if hashes == 0 || hash_start == 0 {
        return;
    }
    // If an odd number of backslashes immediately precedes the run, its first `#`
    // is escaped (literal) and not part of the closing sequence — drop it.
    let backslashes = bytes[..hash_start]
        .iter()
        .rev()
        .take_while(|byte| **byte == b'\\')
        .count();
    if backslashes % 2 == 1 {
        hash_start += 1;
        hashes -= 1;
        if hashes == 0 {
            return;
        }
    }
    // A space/tab before the hashes means the closing sequence is well-formed.
    if matches!(bytes[hash_start - 1], b' ' | b'\t') {
        return;
    }
    let hash_start_abs = span.start + hash_start;
    let report_start = source_text[..hash_start_abs]
        .chars()
        .next_back()
        .map_or(hash_start_abs, |ch| hash_start_abs - ch.len_utf8());
    diagnostics.push(Diagnostic {
        rule_name: "no-missing-atx-heading-space",
        message_id: "missingSpace",
        data: DiagnosticData {
            position: Some(CompactString::from("before")),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(
            source_text,
            ByteSpan {
                start: report_start,
                end: hash_start_abs + hashes,
            },
        ),
        fix: Some(DiagnosticFix {
            start: utf16_offset(source_text, hash_start_abs),
            end: utf16_offset(source_text, hash_start_abs),
            replacement: CompactString::from(" "),
        }),
    });
}

fn scan_missing_label_refs(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // (trimmed label, label-group span). Collected from text nodes; later filtered
    // by allowLabels (raw match) and by any definition whose identifier equals the
    // label (upstream removes references that turn out to have a definition).
    let mut missing: SmallVec<[(CompactString, ByteSpan); 16]> = SmallVec::new();
    let mut definitions = FastHashSet::<CompactString>::default();
    visit_mdast(tree, &mut |node| match node {
        mdast::Node::Definition(definition) => {
            definitions.insert(normalize_identifier(
                definition.label.as_deref().unwrap_or(""),
            ));
        }
        mdast::Node::Text(_) => {
            if let Some(span) = node_span(node) {
                find_missing_references(source_text, span, &mut missing);
            }
        }
        _ => {}
    });

    for (label, span) in &missing {
        if definitions.contains(label) || options.allow_labels.iter().any(|allow| allow == label) {
            continue;
        }
        diagnostics.push(Diagnostic {
            rule_name: "no-missing-label-refs",
            message_id: "notFound",
            data: DiagnosticData {
                label: Some(label.clone()),
                ..DiagnosticData::default()
            },
            loc: line_index.loc_for_span(source_text, *span),
            fix: None,
        });
    }
}

// Upstream `findMissingReferences`: scan a text node for `[left]` / `[left][right]`
// shorthand reference syntax, reporting the effective label (right when present
// and non-empty, else left) at the label group's span.
fn find_missing_references(
    source_text: &str,
    span: ByteSpan,
    missing: &mut SmallVec<[(CompactString, ByteSpan); 16]>,
) {
    // markdown-rs drops the first `\` of a leading escaped backslash from the text
    // node span; extend back over any backslashes so the escape count before a `[`
    // matches the real source (micromark keeps them in the node text).
    let mut start = span.start;
    while start > 0 && source_text.as_bytes()[start - 1] == b'\\' {
        start -= 1;
    }
    let node_text = source_text.get(start..span.end).unwrap_or("");
    let Ok(label_re) = Regex::new(r"\[((?:\\.|[^\[\]\\])*)\](?:\[((?:\\.|[^\]\\])*)\])?") else {
        return;
    };
    // `illegalShorthandTailPattern`: `][<whitespace>]` — handled by
    // no-invalid-label-refs, so skip it here.
    let illegal = Regex::new(r"\]\[\s+\]$").ok();
    for caps in label_re.captures_iter(node_text) {
        let Some(whole) = caps.get(0) else {
            continue;
        };
        // The opening `[` is escaped when preceded by an odd run of backslashes.
        let backslashes = node_text[..whole.start()]
            .bytes()
            .rev()
            .take_while(|byte| *byte == b'\\')
            .count();
        if backslashes % 2 == 1 {
            continue;
        }
        if illegal
            .as_ref()
            .is_some_and(|re| re.is_match(whole.as_str()))
        {
            continue;
        }
        let left = caps.get(1);
        let right = caps.get(2).filter(|group| !group.as_str().is_empty());
        let left_empty = left.is_none_or(|group| group.as_str().is_empty());
        if left_empty && right.is_none() {
            continue;
        }
        let Some(group) = right.or(left) else {
            continue;
        };
        missing.push((
            CompactString::from(group.as_str().trim()),
            ByteSpan {
                start: start + group.start(),
                end: start + group.end(),
            },
        ));
    }
}

fn scan_missing_link_fragments(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let mut ids = FastHashSet::<CompactString>::default();
    let allow_fragment_pattern = options
        .allow_fragment_pattern
        .as_ref()
        .filter(|pattern| !pattern.is_empty())
        .and_then(|pattern| Regex::new(pattern.as_str()).ok());
    ids.insert(CompactString::from("top"));
    for heading in &facts.headings {
        ids.insert(github_slug(heading.text.as_str()));
    }
    for tag in &facts.html_tags {
        if let Some(id) =
            html_attr(tag.raw.as_str(), "id").or_else(|| html_attr(tag.raw.as_str(), "name"))
        {
            ids.insert(if options.ignore_fragment_case {
                lower(id)
            } else {
                CompactString::from(id)
            });
        }
    }
    for link in &facts.links {
        let Some(fragment) = link.url.strip_prefix('#') else {
            continue;
        };
        if fragment.is_empty() || is_github_line_ref(fragment) {
            continue;
        }
        if allow_fragment_pattern
            .as_ref()
            .is_some_and(|pattern| pattern.is_match(fragment))
        {
            continue;
        }
        let normalized = if options.ignore_fragment_case {
            lower(fragment)
        } else {
            CompactString::from(fragment)
        };
        if !ids.contains(&normalized) {
            diagnostics.push(Diagnostic {
                rule_name: "no-missing-link-fragments",
                message_id: "invalidFragment",
                data: DiagnosticData {
                    fragment: Some(CompactString::from(fragment)),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, link.span),
                fix: None,
            });
        }
    }
}

fn scan_multiple_h1(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let mut count = u32::from(facts.frontmatter_has_title);
    for heading in &facts.headings {
        if heading.depth == 1 {
            count += 1;
            if count > 1 {
                diagnostics.push(Diagnostic {
                    rule_name: "no-multiple-h1",
                    message_id: "multipleH1",
                    data: DiagnosticData::default(),
                    loc: line_index.loc_for_span(source_text, heading.span),
                    fix: None,
                });
            }
        }
    }
    for tag in &facts.html_tags {
        if tag.name.eq_ignore_ascii_case("h1") {
            count += 1;
            if count > 1 {
                diagnostics.push(Diagnostic {
                    rule_name: "no-multiple-h1",
                    message_id: "multipleH1",
                    data: DiagnosticData::default(),
                    loc: line_index.loc_for_span(source_text, tag.span),
                    fix: None,
                });
            }
        }
    }
}

fn scan_reference_like_urls(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let definitions = definition_ids(facts, false);
    for link in &facts.links {
        let normalized = normalize_identifier(link.url.as_str());
        if definitions.contains(&normalized) {
            diagnostics.push(Diagnostic {
                rule_name: "no-reference-like-urls",
                message_id: "referenceLikeUrl",
                data: DiagnosticData {
                    link_type: Some(CompactString::from(if link.is_image {
                        "image"
                    } else {
                        "link"
                    })),
                    prefix: Some(CompactString::from(if link.is_image { "!" } else { "" })),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, link.span),
                fix: Some(DiagnosticFix {
                    start: utf16_offset(source_text, link.span.start),
                    end: utf16_offset(source_text, link.span.end),
                    replacement: {
                        let mut out = CompactString::new("");
                        if link.is_image {
                            out.push('!');
                        }
                        out.push('[');
                        out.push_str(link.label.as_str());
                        out.push_str("][");
                        out.push_str(link.url.as_str());
                        out.push(']');
                        out
                    },
                }),
            });
        }
    }
}

fn scan_reversed_media(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let Ok(reversed_re) = Regex::new(r"\(([^()\n]+)\)\[([^\]\n]+)\]") else {
        return;
    };
    for line in &facts.lines {
        if line.in_fence {
            continue;
        }
        for captures in reversed_re.captures_iter(line.text) {
            let Some(mat) = captures.get(0) else {
                continue;
            };
            let label = captures.get(1).map(|value| value.as_str()).unwrap_or("");
            let url = captures.get(2).map(|value| value.as_str()).unwrap_or("");
            let start = line.start + mat.start();
            let end = line.start + mat.end();
            diagnostics.push(Diagnostic {
                rule_name: "no-reversed-media-syntax",
                message_id: "reversedSyntax",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, ByteSpan { start, end }),
                fix: Some(DiagnosticFix {
                    start: utf16_offset(source_text, start),
                    end: utf16_offset(source_text, end),
                    replacement: {
                        let mut out = CompactString::from("[");
                        out.push_str(label);
                        out.push_str("](");
                        out.push_str(url);
                        out.push(')');
                        out
                    },
                }),
            });
        }
    }
}

fn scan_space_in_emphasis(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // (highlight-start offset for sorting, Diagnostic). ESLint reports messages
    // sorted by position, so collect then sort.
    let mut found: SmallVec<[(usize, Diagnostic); 16]> = SmallVec::new();
    visit_mdast(tree, &mut |node| {
        if matches!(
            node,
            mdast::Node::Heading(_) | mdast::Node::Paragraph(_) | mdast::Node::TableCell(_)
        ) {
            scan_emphasis_container(
                source_text,
                line_index,
                node,
                options.check_strikethrough,
                &mut found,
            );
        }
    });
    found.sort_by_key(|(offset, _)| *offset);
    diagnostics.extend(found.into_iter().map(|(_, diagnostic)| diagnostic));
}

// Emphasis markers grouped by `(marker char, length)` to their `(start, end)`
// source spans, in first-seen order.
type MarkerGroups = SmallVec<[((u8, usize), SmallVec<[(usize, usize); 4]>); 6]>;

fn scan_emphasis_container(
    source_text: &str,
    line_index: &LineIndex,
    node: &mdast::Node,
    check_strikethrough: bool,
    found: &mut SmallVec<[(usize, Diagnostic); 16]>,
) {
    let Some(span) = node_span(node) else {
        return;
    };
    // Mask the container source so only the *direct* text children remain (real
    // emphasis/links/etc. become spaces); emphasis-like markers that survive
    // here were NOT parsed as emphasis (typically because of surrounding space).
    let mut buffer = SmallVec::<[u8; 256]>::from_elem(b' ', span.end - span.start);
    if let Some(children) = node.children() {
        for child in children {
            if matches!(child, mdast::Node::Text(_))
                && let Some(child_span) = node_span(child)
            {
                let bytes = source_text.as_bytes();
                let rel = child_span.start - span.start;
                if let Some(slot) = buffer.get_mut(rel..rel + (child_span.end - child_span.start)) {
                    slot.copy_from_slice(&bytes[child_span.start..child_span.end]);
                }
            }
        }
    }

    // Scan emphasis markers in the masked buffer, grouped by marker (char, len).
    let mut groups: MarkerGroups = SmallVec::new();
    let mut index = 0;
    while index < buffer.len() {
        let ch = buffer[index];
        let is_marker = ch == b'*' || ch == b'_' || (check_strikethrough && ch == b'~');
        if is_marker {
            let backslashes = buffer[..index]
                .iter()
                .rev()
                .take_while(|byte| **byte == b'\\')
                .count();
            if backslashes % 2 == 0 {
                let run = buffer[index..]
                    .iter()
                    .take_while(|byte| **byte == ch)
                    .count();
                let len = run.min(if ch == b'~' { 2 } else { 3 });
                let start = span.start + index;
                let key = (ch, len);
                match groups.iter_mut().find(|(group_key, _)| *group_key == key) {
                    Some((_, positions)) => positions.push((start, start + len)),
                    None => {
                        let mut positions = SmallVec::new();
                        positions.push((start, start + len));
                        groups.push((key, positions));
                    }
                }
                index += len;
                continue;
            }
        }
        index += 1;
    }

    // For each marker group, treat consecutive markers as open/close pairs and
    // flag a space just inside either marker.
    for (_, positions) in &groups {
        let mut pair = 0;
        while pair + 1 < positions.len() {
            let (open_start, open_end) = positions[pair];
            report_emphasis_space(
                source_text,
                line_index,
                open_end,
                open_start,
                open_end + 2,
                found,
            );
            let (close_start, close_end) = positions[pair + 1];
            report_emphasis_space(
                source_text,
                line_index,
                close_start.wrapping_sub(1),
                close_start.saturating_sub(2),
                close_end,
                found,
            );
            pair += 2;
        }
    }
}

// Report a space-around-marker violation when `check_offset` is a space/tab:
// highlight `[highlight_start, highlight_end]` and remove the single space.
fn report_emphasis_space(
    source_text: &str,
    line_index: &LineIndex,
    check_offset: usize,
    highlight_start: usize,
    highlight_end: usize,
    found: &mut SmallVec<[(usize, Diagnostic); 16]>,
) {
    if !source_text
        .as_bytes()
        .get(check_offset)
        .is_some_and(|byte| matches!(byte, b' ' | b'\t'))
    {
        return;
    }
    found.push((
        highlight_start,
        Diagnostic {
            rule_name: "no-space-in-emphasis",
            message_id: "spaceInEmphasis",
            data: DiagnosticData::default(),
            loc: line_index.loc_for_span(
                source_text,
                ByteSpan {
                    start: highlight_start,
                    end: highlight_end,
                },
            ),
            fix: Some(DiagnosticFix {
                start: utf16_offset(source_text, check_offset),
                end: utf16_offset(source_text, check_offset + 1),
                replacement: CompactString::new(""),
            }),
        },
    ));
}

fn scan_unused_definitions(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    // (identifier, label-for-data, span, is_footnote)
    let mut defs: SmallVec<[(CompactString, CompactString, ByteSpan, bool); 16]> = SmallVec::new();
    let mut used = FastHashSet::<CompactString>::default();
    let mut used_footnotes = FastHashSet::<CompactString>::default();

    visit_mdast(tree, &mut |node| match node {
        mdast::Node::LinkReference(reference) => {
            used.insert(reference_identifier(
                reference.label.as_deref(),
                &reference.identifier,
            ));
        }
        mdast::Node::ImageReference(reference) => {
            used.insert(reference_identifier(
                reference.label.as_deref(),
                &reference.identifier,
            ));
        }
        mdast::Node::FootnoteReference(reference) => {
            used_footnotes.insert(reference_identifier(
                reference.label.as_deref(),
                &reference.identifier,
            ));
        }
        mdast::Node::Definition(definition) => {
            if let Some(span) = node_span(node) {
                let label = definition.label.as_deref().unwrap_or("");
                defs.push((
                    normalize_identifier(label),
                    CompactString::from(label.trim()),
                    span,
                    false,
                ));
            }
        }
        mdast::Node::FootnoteDefinition(definition) => {
            if let Some(span) = node_span(node) {
                let label = definition.label.as_deref().unwrap_or("");
                defs.push((
                    normalize_identifier(label),
                    CompactString::from(label),
                    span,
                    true,
                ));
            }
        }
        _ => {}
    });

    for (identifier, label, span, is_footnote) in &defs {
        let (allow, used_set, message_id) = if *is_footnote {
            if !options.check_footnote_definitions {
                continue;
            }
            (
                &options.allow_footnote_definitions,
                &used_footnotes,
                "unusedFootnoteDefinition",
            )
        } else {
            (&options.allow_definitions, &used, "unusedDefinition")
        };
        if !is_allowed(allow, identifier) && !used_set.contains(identifier) {
            diagnostics.push(Diagnostic {
                rule_name: "no-unused-definitions",
                message_id,
                data: DiagnosticData {
                    identifier: Some(identifier.clone()),
                    label: Some(label.clone()),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, *span),
                fix: None,
            });
        }
    }
}

// The normalized identifier of a reference, from its label (falling back to the
// mdast identifier when a label is absent), kept consistent with how definition
// identifiers are normalized.
fn reference_identifier(label: Option<&str>, identifier: &str) -> CompactString {
    normalize_identifier(label.unwrap_or(identifier))
}

fn scan_require_alt_text(
    source_text: &str,
    line_index: &LineIndex,
    tree: &mdast::Node,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    visit_mdast(tree, &mut |node| match node {
        // Markdown images / image references: flag when alt text is blank.
        mdast::Node::Image(image) if image.alt.trim().is_empty() => {
            push_alt_required(source_text, line_index, node_span(node), diagnostics);
        }
        mdast::Node::ImageReference(image) if image.alt.trim().is_empty() => {
            push_alt_required(source_text, line_index, node_span(node), diagnostics);
        }
        // Raw HTML: scan each `<img>` tag for a usable alt attribute.
        mdast::Node::Html(_) => {
            if let Some(span) = node_span(node) {
                scan_html_img_alt(source_text, line_index, span, diagnostics);
            }
        }
        _ => {}
    });
}

fn push_alt_required(
    source_text: &str,
    line_index: &LineIndex,
    span: Option<ByteSpan>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    if let Some(span) = span {
        diagnostics.push(Diagnostic {
            rule_name: "require-alt-text",
            message_id: "altTextRequired",
            data: DiagnosticData::default(),
            loc: line_index.loc_for_span(source_text, span),
            fix: None,
        });
    }
}

// Mirror upstream's `<img>` handling: strip comments, then for each img tag flag
// it unless it carries a usable alt. `aria-hidden="true"` exempts the tag. An alt
// attribute that is present but whitespace-only (and non-empty, e.g. `alt=" "`)
// is flagged; a bare `alt` or `alt=""` is accepted.
fn scan_html_img_alt(
    source_text: &str,
    line_index: &LineIndex,
    span: ByteSpan,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let Ok(img_re) = Regex::new(r#"(?i)<img(?:\s(?:[^>"']|"[^"]*"|'[^']*')*)?/?>"#) else {
        return;
    };
    let text = strip_html_comments(source_text.get(span.start..span.end).unwrap_or(""));
    for mat in img_re.find_iter(&text) {
        let tag = mat.as_str();
        if html_attribute(tag, "aria-hidden")
            .flatten()
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        {
            continue;
        }
        let flag = match html_attribute(tag, "alt") {
            None => true,
            Some(Some(value)) => value.trim().is_empty() && !value.is_empty(),
            Some(None) => false,
        };
        if flag {
            let start = span.start + mat.start();
            diagnostics.push(Diagnostic {
                rule_name: "require-alt-text",
                message_id: "altTextRequired",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(
                    source_text,
                    ByteSpan {
                        start,
                        end: start + tag.len(),
                    },
                ),
                fix: None,
            });
        }
    }
}

// Match an HTML attribute the way upstream's `getHtmlAttributeRe` does:
// `\s<name>(\s*=\s*['"]([^'"]*)['"])?`. Returns `None` if absent, `Some(None)` if
// present without a value (bare), `Some(Some(value))` if present with a value.
fn html_attribute(tag: &str, name: &str) -> Option<Option<CompactString>> {
    let pattern = format_attr_pattern(name);
    let attr_re = Regex::new(&pattern).ok()?;
    let caps = attr_re.captures(tag)?;
    Some(caps.get(1).map(|value| CompactString::from(value.as_str())))
}

fn format_attr_pattern(name: &str) -> CompactString {
    let mut pattern = CompactString::from(r"(?i)\s");
    pattern.push_str(name);
    pattern.push_str(r#"(?:\s*=\s*['"]([^'"]*)['"])?"#);
    pattern
}

fn scan_table_column_count(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for index in 1..facts.lines.len() {
        let sep = &facts.lines[index];
        if sep.in_fence || !sep.text.contains('|') || !looks_like_table_separator(sep.text) {
            continue;
        }
        let header = &facts.lines[index - 1];
        if header.in_fence || header.text.trim().is_empty() {
            continue;
        }
        let expected = count_table_cells(header.text);
        // GFM only forms a table when the delimiter row's column count matches
        // the header row's.
        if count_table_cells(sep.text) != expected {
            continue;
        }
        let mut row_index = index + 1;
        while row_index < facts.lines.len() {
            let row = &facts.lines[row_index];
            if row.in_fence || row.text.trim().is_empty() || !row.text.contains('|') {
                break;
            }
            let starts = table_cell_starts(row.text);
            let actual = starts.len();
            if actual > expected {
                // extraCells: from the first cell beyond the header count to the
                // end of the row (`firstExtraCell.start` .. `lastCell.end`).
                let cell_start = row.start + starts[expected];
                diagnostics.push(Diagnostic {
                    rule_name: "table-column-count",
                    message_id: "extraCells",
                    data: DiagnosticData {
                        expected_cells: Some(expected as u32),
                        actual_cells: Some(actual as u32),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(
                        source_text,
                        ByteSpan {
                            start: cell_start,
                            end: row.end,
                        },
                    ),
                    fix: None,
                });
            } else if options.check_missing_table_cells && actual < expected {
                // missingCells: the last column of the row (`lastCell.end - 1` ..
                // row end).
                let start = row.end.saturating_sub(1).max(row.start);
                diagnostics.push(Diagnostic {
                    rule_name: "table-column-count",
                    message_id: "missingCells",
                    data: DiagnosticData {
                        expected_cells: Some(expected as u32),
                        actual_cells: Some(actual as u32),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(
                        source_text,
                        ByteSpan {
                            start,
                            end: row.end,
                        },
                    ),
                    fix: None,
                });
            }
            row_index += 1;
        }
    }
}

fn definition_diagnostic(
    rule_name: &'static str,
    message_id: &'static str,
    source_text: &str,
    line_index: &LineIndex,
    definition: &DefinitionFact,
    first: Option<&DefinitionFact>,
) -> Diagnostic {
    Diagnostic {
        rule_name,
        message_id,
        data: DiagnosticData {
            identifier: Some(definition.identifier.clone()),
            label: Some(definition.label.clone()),
            first_line: first.map(|definition| definition.line),
            first_label: first.map(|definition| definition.label.clone()),
            ..DiagnosticData::default()
        },
        loc: line_index.loc_for_span(source_text, definition.span),
        fix: None,
    }
}

fn definition_ids(facts: &MarkdownFacts<'_>, footnotes: bool) -> FastHashSet<CompactString> {
    facts
        .definitions
        .iter()
        .filter(|definition| definition.is_footnote == footnotes)
        .map(|definition| definition.identifier.clone())
        .collect()
}

fn is_allowed(values: &[CompactString], identifier: &CompactString) -> bool {
    values
        .iter()
        .map(|value| normalize_identifier(value.as_str()))
        .any(|value| value == *identifier)
}

fn split_indent(line: &str) -> (usize, &str) {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    (indent, &line[indent..])
}

// Leading-whitespace width in columns, expanding tabs to the next multiple of 4
// (CommonMark tab stop). A width >= 4 marks an indented code block.
fn indent_columns(line: &str) -> usize {
    let mut columns = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => columns += 1,
            b'\t' => columns += 4 - (columns % 4),
            _ => break,
        }
    }
    columns
}

fn split_fence_info(info: &str) -> (Option<CompactString>, Option<CompactString>) {
    let trimmed = info.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    let mut parts = trimmed.splitn(2, char::is_whitespace);
    let lang = parts
        .next()
        .filter(|part| !part.is_empty())
        .map(CompactString::from);
    let meta = parts
        .next()
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(CompactString::from);
    (lang, meta)
}

fn parse_atx_heading(line: &str) -> Option<(u8, &str)> {
    let (indent, trimmed) = split_indent(line);
    if indent > 3 {
        return None;
    }
    let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let after = trimmed.as_bytes().get(hashes).copied();
    if !matches!(after, Some(b' ' | b'\t') | None) {
        return None;
    }
    let mut text = trimmed[hashes..].trim();
    if text.ends_with('#') {
        text = text.trim_end_matches('#').trim_end();
    }
    Some((hashes as u8, text))
}

fn is_definition_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with('[') && trimmed.contains("]:")
}

fn parse_inline_link(raw: &str, absolute_start: usize) -> Option<LinkFact> {
    let is_image = raw.starts_with("!");
    let label_open = usize::from(is_image);
    let label_close = raw[label_open + 1..].find(']')? + label_open + 1;
    let url_open = raw[label_close + 1..].find('(')? + label_close + 1;
    let url_close = raw.rfind(')')?;
    let label = &raw[label_open + 1..label_close];
    let url = raw[url_open + 1..url_close].trim();
    Some(LinkFact {
        is_image,
        label: CompactString::from(label),
        url: CompactString::from(url.trim_matches(['<', '>'])),
        span: ByteSpan {
            start: absolute_start,
            end: absolute_start + raw.len(),
        },
        url_span: ByteSpan {
            start: absolute_start + url_open + 1,
            end: absolute_start + url_close,
        },
    })
}

fn parse_label_ref(raw: &str, absolute_start: usize, absolute_end: usize) -> Option<LabelRefFact> {
    let is_image = raw.starts_with('!');
    let start = usize::from(is_image);
    let first_close = raw[start + 1..].find(']')? + start + 1;
    let is_footnote = raw[start + 1..].starts_with('^');
    let label = if raw.as_bytes().get(first_close + 1) == Some(&b'[') {
        let second_close = raw[first_close + 2..].find(']')? + first_close + 2;
        &raw[first_close + 2..second_close]
    } else {
        &raw[start + 1..first_close]
    };
    Some(LabelRefFact {
        label: CompactString::from(label.trim_start_matches('^')),
        span: ByteSpan {
            start: absolute_start,
            end: absolute_end,
        },
        is_footnote,
    })
}

fn trim_definition_url(url: &str) -> &str {
    url.trim().trim_matches(['<', '>'])
}

fn normalize_identifier(value: &str) -> CompactString {
    let mut out = CompactString::new("");
    let mut previous_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            if !previous_space {
                out.push(' ');
                previous_space = true;
            }
        } else {
            // Mirror micromark's `normalizeIdentifier(...).toLowerCase()`, i.e.
            // `.toLowerCase().toUpperCase().toLowerCase()`, so case-folds such as
            // ß/ẞ -> "ss" match upstream's identifiers.
            for lowered in ch.to_lowercase() {
                for uppered in lowered.to_uppercase() {
                    for refolded in uppered.to_lowercase() {
                        out.push(refolded);
                    }
                }
            }
            previous_space = false;
        }
    }
    out
}

fn lower(value: &str) -> CompactString {
    let mut out = CompactString::new("");
    for ch in value.chars() {
        for lower in ch.to_lowercase() {
            out.push(lower);
        }
    }
    out
}

fn html_tag_name(raw: &str) -> CompactString {
    let start = usize::from(raw.starts_with("</")) + 1;
    let rest = raw.get(start..).unwrap_or("");
    let end = rest
        .find(|ch: char| ch.is_whitespace() || matches!(ch, '>' | '/'))
        .unwrap_or(rest.len());
    CompactString::from(&rest[..end])
}

fn html_attr<'a>(raw: &'a str, attr: &str) -> Option<&'a str> {
    let mut search = raw;
    while let Some(index) = find_ignore_ascii_case(search, attr) {
        let after = &search[index + attr.len()..];
        let trimmed = after.trim_start();
        if !trimmed.starts_with('=') {
            search = trimmed;
            continue;
        }
        let value = trimmed[1..].trim_start();
        if let Some(rest) = value.strip_prefix('"') {
            return rest.split('"').next();
        }
        if let Some(rest) = value.strip_prefix('\'') {
            return rest.split('\'').next();
        }
        return value
            .split(|ch: char| ch.is_whitespace() || ch == '>')
            .next();
    }
    None
}

fn find_ignore_ascii_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
}

fn in_fenced_range(facts: &MarkdownFacts<'_>, offset: usize) -> bool {
    facts
        .lines
        .iter()
        .any(|line| line.in_fence && offset >= line.start && offset <= line.end)
}

fn github_slug(value: &str) -> CompactString {
    let mut out = CompactString::new("");
    let mut previous_dash = false;
    for ch in value.trim().chars() {
        if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            previous_dash = false;
        } else if (ch.is_whitespace() || ch == '-') && !previous_dash && !out.is_empty() {
            out.push('-');
            previous_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}

fn is_github_line_ref(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('L') else {
        return false;
    };
    rest.chars().next().is_some_and(|ch| ch.is_ascii_digit())
}

// A GFM delimiter row: cells separated by `|` (outer pipes optional), each cell
// matching `:?-+:?` (at least one dash, optional alignment colons).
fn looks_like_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return false;
    }
    let core = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let core = core.strip_suffix('|').unwrap_or(core);
    let mut saw_cell = false;
    for cell in core.split('|') {
        saw_cell = true;
        let bytes = cell.trim().as_bytes();
        let mut index = 0;
        if bytes.first() == Some(&b':') {
            index += 1;
        }
        let dash_start = index;
        while bytes.get(index) == Some(&b'-') {
            index += 1;
        }
        if index == dash_start {
            return false; // no dash
        }
        if bytes.get(index) == Some(&b':') {
            index += 1;
        }
        if index != bytes.len() {
            return false;
        }
    }
    saw_cell
}

// Byte offset (relative to the line) of each table cell's start, matching mdast's
// `tableCell.position.start`: with a leading pipe each pipe begins a cell (the
// trailing pipe excepted); without one, the first cell begins at the first
// non-whitespace character. The count of cells is the returned length.
fn table_cell_starts(line: &str) -> SmallVec<[usize; 8]> {
    let pipes: SmallVec<[usize; 8]> = line
        .bytes()
        .enumerate()
        .filter_map(|(index, byte)| (byte == b'|').then_some(index))
        .collect();
    let first_nonws = line.find(|ch: char| !ch.is_whitespace());
    let last_nonws = line.rfind(|ch: char| !ch.is_whitespace());
    let has_leading = first_nonws.is_some_and(|index| line.as_bytes()[index] == b'|');
    let has_trailing = last_nonws.is_some_and(|index| line.as_bytes()[index] == b'|');
    let kept = pipes.len() - usize::from(has_trailing);
    let mut starts = SmallVec::new();
    if !has_leading {
        starts.push(first_nonws.unwrap_or(0));
    }
    starts.extend(pipes.into_iter().take(kept));
    starts
}

fn count_table_cells(line: &str) -> usize {
    table_cell_starts(line).len()
}

fn utf16_offset(source_text: &str, byte_offset: usize) -> u32 {
    let mut byte_offset = byte_offset.min(source_text.len());
    // Defensive: floor to a char boundary so slicing never panics on non-ASCII.
    while byte_offset > 0 && !source_text.is_char_boundary(byte_offset) {
        byte_offset -= 1;
    }
    source_text[..byte_offset]
        .chars()
        .map(char::len_utf16)
        .sum::<usize>() as u32
}

struct LineIndex {
    line_starts: SmallVec<[usize; 64]>,
}

impl LineIndex {
    fn new(source_text: &str) -> Self {
        // A line break is `\r\n`, `\r`, or `\n` (ESLint / mdast line counting).
        let mut line_starts = SmallVec::new();
        line_starts.push(0);
        let bytes = source_text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => line_starts.push(index + 1),
                b'\r' => {
                    if bytes.get(index + 1) == Some(&b'\n') {
                        index += 1;
                    }
                    line_starts.push(index + 1);
                }
                _ => {}
            }
            index += 1;
        }
        Self { line_starts }
    }

    fn loc_for_span(&self, source_text: &str, span: ByteSpan) -> DiagnosticLoc {
        let (start_line, start_column) = self.position_for_offset(source_text, span.start);
        let (end_line, end_column) = self.position_for_offset(source_text, span.end);
        DiagnosticLoc {
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    fn position_for_offset(&self, source_text: &str, offset: usize) -> (u32, u32) {
        let mut offset = offset.min(source_text.len());
        // Defensive: a rule may hand us a byte offset that lands inside a
        // multibyte char on non-ASCII input; floor it to the nearest char
        // boundary so the slice below never panics.
        while offset > 0 && !source_text.is_char_boundary(offset) {
            offset -= 1;
        }
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

#[cfg(test)]
mod tests {
    use super::{RULE_NAMES, ScanOptions, scan_eslint_markdown};

    #[test]
    fn exposes_all_rule_names() {
        assert_eq!(RULE_NAMES.len(), 21);
        assert!(RULE_NAMES.contains(&"fenced-code-language"));
        assert!(RULE_NAMES.contains(&"table-column-count"));
    }

    #[test]
    fn scans_representative_rules() {
        let source = [
            "```",
            "code",
            "```",
            "",
            "# Title",
            "### Skipped",
            "# Title",
            "",
            "[foo]: #",
            "[foo]: https://example.com",
            "[missing][nope]",
            "[empty]()",
            "![](#)",
            "(label)[https://example.com]",
            "https://example.com",
            "<div><img src=\"x\"></div>",
            "| a | b |",
            "| --- | --- |",
            "| 1 | 2 | 3 |",
        ]
        .join("\n");
        let diagnostics = scan_eslint_markdown(&source, &ScanOptions::default());
        let rule_names: oxlint_plugins_carton::SmallVec<[&str; 24]> = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.rule_name)
            .collect();

        assert!(rule_names.contains(&"fenced-code-language"));
        assert!(rule_names.contains(&"heading-increment"));
        assert!(rule_names.contains(&"no-duplicate-headings"));
        assert!(rule_names.contains(&"no-empty-definitions"));
        assert!(rule_names.contains(&"no-empty-links"));
        assert!(rule_names.contains(&"no-empty-images"));
        assert!(rule_names.contains(&"no-reversed-media-syntax"));
        assert!(rule_names.contains(&"no-bare-urls"));
        assert!(rule_names.contains(&"no-html"));
        assert!(rule_names.contains(&"require-alt-text"));
        assert!(rule_names.contains(&"table-column-count"));
    }
}
