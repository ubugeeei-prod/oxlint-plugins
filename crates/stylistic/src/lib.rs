use oxlint_plugins_carton::SmallVec;

pub const DEFAULT_FORBIDDEN_NAMES: &[&str] = &["event", "error", "data"];
static DEFAULT_FORBIDDEN_NAME_SET: phf::Set<&'static str> = phf::phf_set! {
    "event",
    "error",
    "data",
};

pub fn scan_source_for_rule<'a>(
    source_text: &str,
    custom_names: impl IntoIterator<Item = &'a str>,
) -> SmallVec<[&'a str; 8]> {
    let mut matches = SmallVec::new();

    for name in forbidden_names(custom_names) {
        if contains_identifier(source_text, name) {
            matches.push(name);
        }
    }

    matches
}

pub fn is_forbidden_identifier_name<'a>(
    name: &str,
    custom_names: impl IntoIterator<Item = &'a str>,
) -> bool {
    custom_names
        .into_iter()
        .filter(|custom_name| !custom_name.is_empty())
        .any(|forbidden| forbidden == name)
        || DEFAULT_FORBIDDEN_NAME_SET.contains(name)
}

pub fn contains_identifier(source_text: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return false;
    }

    let bytes = source_text.as_bytes();
    let mut offset = 0;

    while let Some(relative_start) = source_text[offset..].find(needle) {
        let start = offset + relative_start;
        let end = start + needle.len();

        if has_identifier_boundaries(bytes, start, end) {
            return true;
        }

        offset = end;
    }

    false
}

fn forbidden_names<'a>(
    custom_names: impl IntoIterator<Item = &'a str>,
) -> impl Iterator<Item = &'a str> {
    custom_names
        .into_iter()
        .filter(|name| !name.is_empty())
        .chain(DEFAULT_FORBIDDEN_NAMES.iter().copied())
}

fn has_identifier_boundaries(bytes: &[u8], start: usize, end: usize) -> bool {
    let before_is_identifier = start
        .checked_sub(1)
        .and_then(|index| bytes.get(index))
        .is_some_and(|byte| is_ascii_identifier_continue(*byte));
    let after_is_identifier = bytes
        .get(end)
        .is_some_and(|byte| is_ascii_identifier_continue(*byte));

    !before_is_identifier && !after_is_identifier
}

fn is_ascii_identifier_continue(byte: u8) -> bool {
    byte == b'_' || byte == b'$' || byte.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::{contains_identifier, scan_source_for_rule};

    #[test]
    fn scans_default_names() {
        let source = "const event = data.error;";
        insta::assert_debug_snapshot!(scan_source_for_rule(source, []));
    }

    #[test]
    fn supports_custom_names_without_losing_defaults() {
        let source = "function run(ctx) { return payload + event; }";
        insta::assert_debug_snapshot!(scan_source_for_rule(source, ["ctx", "payload"]));
    }

    #[test]
    fn respects_identifier_boundaries() {
        insta::assert_debug_snapshot!(
            "identifier_boundaries",
            vec![
                contains_identifier("const event = 1", "event"),
                contains_identifier("const eventBus = 1", "event"),
                contains_identifier("const my_event = 1", "event"),
                contains_identifier("const $event = 1", "event"),
                contains_identifier("call(event)", "event"),
            ]
        );
    }
}
