//! Rule `no-clear-text-protocols` (SonarJS key S5332).
//!
//! Clean-room port. Reports string literals that contain a URL using a protocol
//! that sends data without transport encryption. This syntactic implementation
//! intentionally focuses on clear literal URL schemes and does not attempt to
//! resolve concatenated strings, template literals, or runtime values.
//!
//! **Flagged**:
//! - `"http://example.com"` — clear-text HTTP.
//! - `"ftp://files.example.com"` — clear-text FTP.
//! - `"ws://example.com/socket"` — clear-text WebSocket.
//! - `"telnet://host"` — clear-text Telnet.
//!
//! **Not flagged**:
//! - `"https://example.com"` — encrypted HTTP.
//! - `"wss://example.com/socket"` — encrypted WebSocket.
//! - `"http: label"` — no URL authority marker.
//!
//! Behaviour is reproduced from the public RSPEC S5332 description only; no
//! upstream source, tests, fixtures, or message strings were consulted or
//! copied.

use oxc_ast::ast::StringLiteral;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-clear-text-protocols";

const CLEAR_TEXT_PROTOCOLS: [&[u8]; 5] =
    [b"http://", b"ftp://", b"telnet://", b"ws://", b"ldap://"];

fn ascii_window_eq_ignore_case(window: &[u8], needle: &[u8]) -> bool {
    window
        .iter()
        .zip(needle.iter())
        .all(|(left, right)| left.to_ascii_lowercase() == *right)
}

fn ascii_contains_ignore_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| ascii_window_eq_ignore_case(window, needle))
}

fn contains_clear_text_protocol(value: &str) -> bool {
    let bytes = value.as_bytes();
    CLEAR_TEXT_PROTOCOLS
        .iter()
        .any(|scheme| ascii_contains_ignore_case(bytes, scheme))
}

impl Scanner<'_> {
    pub(crate) fn check_no_clear_text_protocols(&mut self, it: &StringLiteral<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        if contains_clear_text_protocol(it.value.as_str()) {
            self.report(RULE_NAME, "clearTextProtocol", it.span);
        }
    }
}
