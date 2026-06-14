#![doc = "Rust implementation of @eslint/markdown rule logic."]

use oxlint_plugins_carton::{CompactString, FastHashMap, FastHashSet, SmallVec};
use regex::Regex;

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

const EMPHASIS_MARKERS: &[&str] = &["***", "**", "*", "___", "__", "_"];
const EMPHASIS_AND_STRIKE_MARKERS: &[&str] = &["***", "**", "*", "___", "__", "_", "~~", "~"];

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
    if options.is_enabled("no-bare-urls") {
        scan_bare_urls(source_text, &line_index, &facts, &mut diagnostics);
    }
    if options.is_enabled("no-duplicate-definitions") {
        scan_duplicate_definitions(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-duplicate-headings") {
        scan_duplicate_headings(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-empty-definitions") {
        scan_empty_definitions(source_text, &line_index, &facts, options, &mut diagnostics);
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
    if options.is_enabled("no-missing-atx-heading-space") {
        scan_missing_atx_heading_space(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-missing-label-refs") {
        scan_missing_label_refs(source_text, &line_index, &facts, options, &mut diagnostics);
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
    if options.is_enabled("no-space-in-emphasis") {
        scan_space_in_emphasis(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("no-unused-definitions") {
        scan_unused_definitions(source_text, &line_index, &facts, options, &mut diagnostics);
    }
    if options.is_enabled("require-alt-text") {
        scan_require_alt_text(source_text, &line_index, &facts, &mut diagnostics);
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
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let Ok(url_re) =
        Regex::new(r#"(?i)\b(?:https?://[^\s<>()]+|[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,})"#)
    else {
        return;
    };
    for line in &facts.lines {
        if line.in_fence || line.text.starts_with("    ") || line.text.starts_with('\t') {
            continue;
        }
        for mat in url_re.find_iter(line.text) {
            let start = line.start + mat.start();
            let mut end = line.start + mat.end();
            while end > start
                && source_text
                    .as_bytes()
                    .get(end - 1)
                    .is_some_and(|byte| matches!(byte, b'.' | b',' | b';' | b':' | b'!' | b'?'))
            {
                end -= 1;
            }
            if end == start {
                continue;
            }
            if is_inside_angle_autolink(source_text, start, end)
                || is_inside_markdown_destination(&facts.links, start)
                || is_inside_html_tag(&facts.html_tags, start)
            {
                continue;
            }
            diagnostics.push(Diagnostic {
                rule_name: "no-bare-urls",
                message_id: "bareUrl",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, ByteSpan { start, end }),
                fix: Some(DiagnosticFix {
                    start: utf16_offset(source_text, start),
                    end: utf16_offset(source_text, end),
                    replacement: {
                        let mut out = CompactString::from("<");
                        out.push_str(&source_text[start..end]);
                        out.push('>');
                        out
                    },
                }),
            });
        }
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
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let mut all_seen = FastHashSet::<CompactString>::default();
    let mut seen_by_parent = FastHashSet::<(usize, u8, CompactString)>::default();
    let mut parent_stack = [0usize; 6];
    let mut headings = facts.headings.iter().collect::<SmallVec<[_; 16]>>();
    headings.sort_by_key(|heading| heading.span.start);
    for (index, heading) in headings.into_iter().enumerate() {
        let key = normalize_heading_text(heading.text.as_str());
        let duplicate = if options.check_duplicate_headings_siblings_only {
            let depth_index = usize::from(heading.depth.saturating_sub(1));
            for value in parent_stack.iter_mut().skip(depth_index) {
                *value = 0;
            }
            let parent_id = if depth_index == 0 {
                0
            } else {
                parent_stack[depth_index - 1]
            };
            parent_stack[depth_index] = index + 1;
            !seen_by_parent.insert((parent_id, heading.depth, key))
        } else {
            !all_seen.insert(key.clone())
        };
        if duplicate {
            diagnostics.push(Diagnostic {
                rule_name: "no-duplicate-headings",
                message_id: "duplicateHeading",
                data: DiagnosticData {
                    text: Some(heading.text.clone()),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, heading.span),
                fix: None,
            });
        }
    }
}

fn scan_empty_definitions(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for definition in &facts.definitions {
        if definition.is_footnote {
            if options.check_footnote_definitions
                && !is_allowed(&options.allow_footnote_definitions, &definition.identifier)
                && definition.url.trim().is_empty()
            {
                diagnostics.push(definition_diagnostic(
                    "no-empty-definitions",
                    "emptyFootnoteDefinition",
                    source_text,
                    line_index,
                    definition,
                    None,
                ));
            }
        } else if !is_allowed(&options.allow_definitions, &definition.identifier)
            && (definition.url.trim().is_empty() || matches!(definition.url.as_str(), "#" | "<>"))
        {
            diagnostics.push(definition_diagnostic(
                "no-empty-definitions",
                "emptyDefinition",
                source_text,
                line_index,
                definition,
                None,
            ));
        }
    }
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
    for label_ref in &facts.label_refs {
        if label_ref.label.trim().is_empty() {
            diagnostics.push(Diagnostic {
                rule_name: "no-invalid-label-refs",
                message_id: "invalidLabelRef",
                data: DiagnosticData {
                    label: Some(label_ref.label.clone()),
                    ..DiagnosticData::default()
                },
                loc: line_index.loc_for_span(source_text, label_ref.span),
                fix: None,
            });
        }
    }
}

fn scan_missing_atx_heading_space(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for line in &facts.lines {
        if line.in_fence {
            continue;
        }
        let (indent, trimmed) = split_indent(line.text);
        if indent > 3 {
            continue;
        }
        let hashes = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        if (1..=6).contains(&hashes) {
            let after = trimmed.as_bytes().get(hashes).copied();
            if !matches!(after, Some(b' ' | b'\t' | b'#')) {
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
                            start: line.start + indent,
                            end: line.start + indent + hashes + usize::from(after.is_some()),
                        },
                    ),
                    fix: Some(DiagnosticFix {
                        start: utf16_offset(source_text, line.start + indent + hashes),
                        end: utf16_offset(source_text, line.start + indent + hashes),
                        replacement: CompactString::from(" "),
                    }),
                });
            }
        }
        if options.check_closed_headings && trimmed.ends_with('#') {
            let before_last = trimmed.trim_end_matches('#').chars().last();
            if before_last.is_some_and(|ch| !ch.is_whitespace()) {
                let end = line.end;
                let start =
                    end.saturating_sub(trimmed.chars().rev().take_while(|ch| *ch == '#').count());
                diagnostics.push(Diagnostic {
                    rule_name: "no-missing-atx-heading-space",
                    message_id: "missingSpace",
                    data: DiagnosticData {
                        position: Some(CompactString::from("before")),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(source_text, ByteSpan { start, end }),
                    fix: Some(DiagnosticFix {
                        start: utf16_offset(source_text, start),
                        end: utf16_offset(source_text, start),
                        replacement: CompactString::from(" "),
                    }),
                });
            }
        }
    }
}

fn scan_missing_label_refs(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let definitions = definition_ids(facts, false);
    for label_ref in &facts.label_refs {
        if label_ref.is_footnote || label_ref.label.trim().is_empty() {
            continue;
        }
        let normalized = normalize_identifier(label_ref.label.as_str());
        if definitions.contains(&normalized) || is_allowed(&options.allow_labels, &label_ref.label)
        {
            continue;
        }
        diagnostics.push(Diagnostic {
            rule_name: "no-missing-label-refs",
            message_id: "notFound",
            data: DiagnosticData {
                label: Some(label_ref.label.clone()),
                ..DiagnosticData::default()
            },
            loc: line_index.loc_for_span(source_text, label_ref.span),
            fix: None,
        });
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
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let markers = if options.check_strikethrough {
        EMPHASIS_AND_STRIKE_MARKERS
    } else {
        EMPHASIS_MARKERS
    };
    for line in &facts.lines {
        if line.in_fence {
            continue;
        }
        for marker in markers {
            let mut search_start = 0;
            while let Some(index) = line.text[search_start..].find(marker) {
                let marker_start = search_start + index;
                let marker_end = marker_start + marker.len();
                let after = line.text.as_bytes().get(marker_end).copied();
                let before = marker_start
                    .checked_sub(1)
                    .and_then(|idx| line.text.as_bytes().get(idx))
                    .copied();
                let violation =
                    matches!(after, Some(b' ' | b'\t')) || matches!(before, Some(b' ' | b'\t'));
                if violation {
                    diagnostics.push(Diagnostic {
                        rule_name: "no-space-in-emphasis",
                        message_id: "spaceInEmphasis",
                        data: DiagnosticData::default(),
                        loc: line_index.loc_for_span(
                            source_text,
                            ByteSpan {
                                start: line.start + marker_start,
                                end: line.start + marker_end,
                            },
                        ),
                        fix: None,
                    });
                }
                search_start = marker_end;
            }
        }
    }
}

fn scan_unused_definitions(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    options: &ScanOptions,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    let used = used_label_ids(facts, false);
    let used_footnotes = used_label_ids(facts, true);
    for definition in &facts.definitions {
        if definition.is_footnote {
            if options.check_footnote_definitions
                && !is_allowed(&options.allow_footnote_definitions, &definition.identifier)
                && !used_footnotes.contains(&definition.identifier)
            {
                diagnostics.push(definition_diagnostic(
                    "no-unused-definitions",
                    "unusedFootnoteDefinition",
                    source_text,
                    line_index,
                    definition,
                    None,
                ));
            }
        } else if !is_allowed(&options.allow_definitions, &definition.identifier)
            && !used.contains(&definition.identifier)
        {
            diagnostics.push(definition_diagnostic(
                "no-unused-definitions",
                "unusedDefinition",
                source_text,
                line_index,
                definition,
                None,
            ));
        }
    }
}

fn scan_require_alt_text(
    source_text: &str,
    line_index: &LineIndex,
    facts: &MarkdownFacts<'_>,
    diagnostics: &mut SmallVec<[Diagnostic; 32]>,
) {
    for link in &facts.links {
        if link.is_image && link.label.trim().is_empty() {
            diagnostics.push(Diagnostic {
                rule_name: "require-alt-text",
                message_id: "altTextRequired",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, link.span),
                fix: None,
            });
        }
    }
    for tag in &facts.html_tags {
        if !tag.name.eq_ignore_ascii_case("img") {
            continue;
        }
        let alt = html_attr(tag.raw.as_str(), "alt");
        let aria_hidden = html_attr(tag.raw.as_str(), "aria-hidden");
        if aria_hidden.is_some_and(|value| value.eq_ignore_ascii_case("true")) {
            continue;
        }
        if alt.is_none_or(|value| value.trim().is_empty()) {
            diagnostics.push(Diagnostic {
                rule_name: "require-alt-text",
                message_id: "altTextRequired",
                data: DiagnosticData::default(),
                loc: line_index.loc_for_span(source_text, tag.span),
                fix: None,
            });
        }
    }
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
        if sep.in_fence || !looks_like_table_separator(sep.text) {
            continue;
        }
        let header = &facts.lines[index - 1];
        let expected = count_table_cells(header.text);
        let mut row_index = index + 1;
        while row_index < facts.lines.len() && facts.lines[row_index].text.contains('|') {
            let row = &facts.lines[row_index];
            if row.text.trim().is_empty() {
                break;
            }
            let actual = count_table_cells(row.text);
            if actual > expected || (options.check_missing_table_cells && actual < expected) {
                diagnostics.push(Diagnostic {
                    rule_name: "table-column-count",
                    message_id: if actual > expected {
                        "extraCells"
                    } else {
                        "missingCells"
                    },
                    data: DiagnosticData {
                        expected_cells: Some(expected as u32),
                        actual_cells: Some(actual as u32),
                        ..DiagnosticData::default()
                    },
                    loc: line_index.loc_for_span(
                        source_text,
                        ByteSpan {
                            start: row.start,
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

fn used_label_ids(facts: &MarkdownFacts<'_>, footnotes: bool) -> FastHashSet<CompactString> {
    facts
        .label_refs
        .iter()
        .filter(|label_ref| label_ref.is_footnote == footnotes)
        .map(|label_ref| normalize_identifier(label_ref.label.as_str()))
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
            for lower in ch.to_lowercase() {
                out.push(lower);
            }
            previous_space = false;
        }
    }
    out
}

fn normalize_heading_text(value: &str) -> CompactString {
    CompactString::from(value.trim())
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

fn is_inside_angle_autolink(source_text: &str, start: usize, end: usize) -> bool {
    source_text[..start].ends_with('<') && source_text[end..].starts_with('>')
}

fn is_inside_markdown_destination(links: &[LinkFact], offset: usize) -> bool {
    links
        .iter()
        .any(|link| offset >= link.url_span.start && offset <= link.url_span.end)
}

fn is_inside_html_tag(tags: &[HtmlTagFact], offset: usize) -> bool {
    tags.iter()
        .any(|tag| offset >= tag.span.start && offset <= tag.span.end)
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

fn looks_like_table_separator(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains('|')
        && trimmed
            .chars()
            .all(|ch| matches!(ch, '|' | '-' | ':' | ' ' | '\t'))
        && trimmed.contains("---")
}

fn count_table_cells(line: &str) -> usize {
    let trimmed = line.trim().trim_matches('|');
    trimmed.split('|').count()
}

fn utf16_offset(source_text: &str, byte_offset: usize) -> u32 {
    source_text[..byte_offset.min(source_text.len())]
        .chars()
        .map(char::len_utf16)
        .sum::<usize>() as u32
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
        let offset = offset.min(source_text.len());
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
