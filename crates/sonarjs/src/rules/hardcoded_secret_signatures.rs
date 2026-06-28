//! Rule `hardcoded-secret-signatures` (SonarJS key S6437).
//!
//! Clean-room port from the public RSPEC S6437 behavioral description only; no
//! upstream eslint-plugin-sonarjs / SonarJS source, tests, fixtures, message
//! strings, or docs were opened, read, or copied. All matching logic, messages,
//! and tests below are authored independently.
//!
//! ## What this rule targets
//! Unlike the name-based hard-coded credential rules (`no-hardcoded-passwords`
//! S2068, `no-hardcoded-secrets` S6418) — which key off a credential-like
//! *variable / property name* assigned a string — this rule (S6437) detects a
//! leaked secret by the **signature of the value itself**: a hard-coded string
//! literal whose contents match the unmistakable, provider-specific token
//! format of a well-known cloud / SaaS credential. The variable name is
//! irrelevant; the value alone identifies the secret.
//!
//! ## Quality philosophy: narrow, false-positive-free subset
//! Generic high-entropy detection is intentionally NOT attempted here because
//! the runtime offers no entropy/dataflow engine and entropy heuristics are
//! prone to false positives. Instead we match only token shapes that are, in
//! practice, impossible to be anything but a real credential: a fixed provider
//! prefix at a word boundary followed by an exact (or generous minimum) run of
//! the provider's alphabet. We deliberately UNDER-report rather than
//! over-report.
//!
//! ## Recognised signatures (matched anywhere inside the string, at a word
//! boundary so embedded `Bearer <token>` forms are still caught):
//!  - AWS access key id: `AKIA`/`ASIA` + exactly 16 uppercase base-32 chars.
//!  - GitHub tokens: `ghp_`/`gho_`/`ghu_`/`ghs_`/`ghr_` + >= 36 alphanumerics,
//!    and `github_pat_` + >= 22 [A-Za-z0-9_].
//!  - Google API key: `AIza` + exactly 35 chars of `[A-Za-z0-9_-]`.
//!  - Slack token: `xoxb-`/`xoxp-`/`xoxa-`/`xoxr-`/`xoxs-`/`xoxo-` + >= 10
//!    chars of `[A-Za-z0-9-]`.
//!  - Stripe secret/restricted key: `sk_live_`/`rk_live_`/`sk_test_`/`rk_test_`
//!    + >= 20 alphanumerics. (Publishable `pk_` keys are NOT secret and are not
//!      > matched.)
//!  - npm access token: `npm_` + exactly 36 alphanumerics.
//!  - PEM private-key block: contains both `-----BEGIN` and `PRIVATE KEY-----`.
//!
//! ## Flagged
//! ```js
//! const a = "AKIAIOSFODNN7EXAMPLE";                       // AWS key id
//! foo("ghp_0123456789abcdefghijklmnopqrstuvwxyz");        // GitHub PAT
//! const k = "sk_live_EXAMPLENOTREALKEY123";              // Stripe secret key
//! ```
//!
//! ## Not flagged
//! ```js
//! const id = "pk_live_4eC39HqLyjWDarjtT1zdp7dc";  // publishable, not secret
//! const s = "hello world";                        // no signature
//! const u = "550e8400-e29b-41d4-a716-446655440000"; // a plain UUID
//! const x = process.env.SECRET;                   // not a string literal
//! ```

use oxc_ast::ast::StringLiteral;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "hardcoded-secret-signatures";

/// A byte that can be part of an identifier-like word, used for boundary tests
/// so that a prefix only matches when it is not glued to preceding word chars.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn is_alnum(b: u8) -> bool {
    b.is_ascii_alphanumeric()
}

/// Uppercase base-32 alphabet used by AWS access key ids (`[A-Z0-9]`, a
/// conservative superset of the true `[A-Z2-7]`).
fn is_upper_alnum(b: u8) -> bool {
    b.is_ascii_uppercase() || b.is_ascii_digit()
}

/// URL-safe base64 alphabet used by Google API keys.
fn is_b64url(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'_'
}

/// Slack token body alphabet.
fn is_slack_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-'
}

/// GitHub fine-grained PAT body alphabet (`github_pat_...`).
fn is_word_body(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Scans `hay` for `prefix` occurring at a left word boundary, followed by a
/// run of `suffix_ok` bytes whose length is in `[suffix_min, suffix_max]`
/// (when `suffix_max == 0` there is no upper bound). The upper bound also
/// serves as a right boundary: an over-long run fails an exact-length
/// signature, preventing a longer look-alike token from matching a shorter
/// pattern.
fn scan_signature(
    hay: &[u8],
    prefix: &[u8],
    suffix_min: usize,
    suffix_max: usize,
    suffix_ok: fn(u8) -> bool,
) -> bool {
    let plen = prefix.len();
    if plen == 0 || hay.len() < plen + suffix_min {
        return false;
    }
    let mut i = 0;
    while i + plen <= hay.len() {
        if &hay[i..i + plen] == prefix {
            let left_ok = i == 0 || !is_word_byte(hay[i - 1]);
            if left_ok {
                let mut j = i + plen;
                while j < hay.len() && suffix_ok(hay[j]) {
                    j += 1;
                }
                let count = j - (i + plen);
                let max_ok = suffix_max == 0 || count <= suffix_max;
                if count >= suffix_min && max_ok {
                    return true;
                }
            }
        }
        i += 1;
    }
    false
}

/// Returns `true` when `value` contains a recognised, high-confidence secret
/// signature.
fn matches_secret_signature(value: &str) -> bool {
    let hay = value.as_bytes();

    // PEM private-key block (RSA / EC / generic / OpenSSH).
    if value.contains("-----BEGIN") && value.contains("PRIVATE KEY-----") {
        return true;
    }

    // AWS access key id: prefix + exactly 16 uppercase base-32 chars.
    for prefix in [b"AKIA".as_slice(), b"ASIA".as_slice()] {
        if scan_signature(hay, prefix, 16, 16, is_upper_alnum) {
            return true;
        }
    }

    // GitHub classic tokens: prefix + >= 36 alphanumerics.
    for prefix in [
        b"ghp_".as_slice(),
        b"gho_".as_slice(),
        b"ghu_".as_slice(),
        b"ghs_".as_slice(),
        b"ghr_".as_slice(),
    ] {
        if scan_signature(hay, prefix, 36, 0, is_alnum) {
            return true;
        }
    }
    // GitHub fine-grained PAT.
    if scan_signature(hay, b"github_pat_", 22, 0, is_word_body) {
        return true;
    }

    // Google API key: `AIza` + exactly 35 url-safe base64 chars.
    if scan_signature(hay, b"AIza", 35, 35, is_b64url) {
        return true;
    }

    // Slack token.
    for prefix in [
        b"xoxb-".as_slice(),
        b"xoxp-".as_slice(),
        b"xoxa-".as_slice(),
        b"xoxr-".as_slice(),
        b"xoxs-".as_slice(),
        b"xoxo-".as_slice(),
    ] {
        if scan_signature(hay, prefix, 10, 0, is_slack_body) {
            return true;
        }
    }

    // Stripe secret / restricted keys (not publishable `pk_`).
    for prefix in [
        b"sk_live_".as_slice(),
        b"rk_live_".as_slice(),
        b"sk_test_".as_slice(),
        b"rk_test_".as_slice(),
    ] {
        if scan_signature(hay, prefix, 20, 0, is_alnum) {
            return true;
        }
    }

    // npm access token: `npm_` + exactly 36 alphanumerics.
    if scan_signature(hay, b"npm_", 36, 36, is_alnum) {
        return true;
    }

    false
}

impl Scanner<'_> {
    pub(crate) fn check_hardcoded_secret_signatures(&mut self, it: &StringLiteral<'_>) {
        if matches_secret_signature(it.value.as_str()) {
            self.report(RULE_NAME, "hardcodedSecret", it.span);
        }
    }
}
