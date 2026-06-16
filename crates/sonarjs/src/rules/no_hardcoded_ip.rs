//! Rule `no-hardcoded-ip` (SonarJS key S1313).
//!
//! Clean-room port. Reports string literals that embed a hardcoded IPv4 or
//! IPv6 address. The following addresses are excluded (not reported):
//!
//!  - IPv4 loopback range 127.0.0.0/8
//!  - IPv4 unspecified address 0.0.0.0
//!  - IPv4 broadcast address 255.255.255.255
//!  - IPv6 loopback ::1
//!  - IPv6 unspecified :: (all zeros, 0:0:0:0:0:0:0:0)
//!  - IPv6 documentation range 2001:db8::/32 (RFC 3849)
//!  - IPv4-mapped loopback ::ffff:127.x.x.x
//!
//! Behaviour derived from public RSPEC S1313 documentation only; no upstream
//! source, tests, fixtures, or message strings were consulted or copied.

use std::net::{Ipv4Addr, Ipv6Addr};

use oxc_ast::ast::StringLiteral;

use crate::scanner::Scanner;

pub(crate) const RULE_NAME: &str = "no-hardcoded-ip";

/// Returns `true` if `c` may appear inside a potential IPv4 address candidate.
#[inline]
fn is_ipv4_char(c: u8) -> bool {
    c.is_ascii_digit() || c == b'.'
}

/// Returns `true` if `c` may appear inside a potential IPv6 address candidate.
#[inline]
fn is_ipv6_char(c: u8) -> bool {
    c.is_ascii_hexdigit() || c == b':' || c == b'.'
}

/// Returns `true` if `addr` should not be reported (non-sensitive IPv4 address).
fn is_excluded_ipv4(addr: Ipv4Addr) -> bool {
    addr.is_loopback() || addr.is_unspecified() || addr.is_broadcast()
}

/// Returns `true` if `candidate` parses as a valid, non-excluded IPv4 address.
fn ipv4_candidate_is_flagged(candidate: &str) -> bool {
    match candidate.parse::<Ipv4Addr>() {
        Ok(addr) => !is_excluded_ipv4(addr),
        Err(_) => false,
    }
}

/// Returns `true` if `candidate` parses as a valid, non-excluded IPv6 address.
fn ipv6_candidate_is_flagged(candidate: &str) -> bool {
    match candidate.parse::<Ipv6Addr>() {
        Ok(addr) => !is_excluded_ipv6(addr),
        Err(_) => false,
    }
}

/// Returns `true` if `addr` should not be reported (non-sensitive IPv6 address).
fn is_excluded_ipv6(addr: Ipv6Addr) -> bool {
    if addr.is_loopback() || addr.is_unspecified() {
        return true;
    }
    // 2001:db8::/32 — documentation range (RFC 3849)
    let segs = addr.segments();
    if segs[0] == 0x2001 && segs[1] == 0x0db8 {
        return true;
    }
    // ::ffff:x.x.x.x — IPv4-mapped; exclude when the mapped IPv4 is excluded.
    if let Some(v4) = addr.to_ipv4_mapped() {
        return is_excluded_ipv4(v4);
    }
    false
}

/// Returns `true` if `s` contains an embedded valid, non-excluded IPv4 address.
fn contains_flagged_ipv4(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    while i < len {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < len && is_ipv4_char(bytes[i]) {
                i += 1;
            }
            let candidate = &s[start..i];
            // Shortest valid IPv4 address is "0.0.0.0" (7 characters).
            if candidate.len() >= 7 {
                let prev_ok = start == 0 || !is_ipv4_char(bytes[start - 1]);
                let next_ok = i >= len || !is_ipv4_char(bytes[i]);
                if prev_ok && next_ok && ipv4_candidate_is_flagged(candidate) {
                    return true;
                }
            }
        } else {
            i += 1;
        }
    }
    false
}

/// Returns `true` if `s` contains an embedded valid, non-excluded IPv6 address.
fn contains_flagged_ipv6(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0usize;
    while i < len {
        if bytes[i].is_ascii_hexdigit() || bytes[i] == b':' {
            let start = i;
            while i < len && is_ipv6_char(bytes[i]) {
                i += 1;
            }
            let candidate = &s[start..i];
            // A valid IPv6 address has at least two colons ("::1" has two).
            let colons = candidate.bytes().filter(|&b| b == b':').count();
            if colons >= 2 {
                let prev_ok = start == 0 || !is_ipv6_char(bytes[start - 1]);
                let next_ok = i >= len || !is_ipv6_char(bytes[i]);
                if prev_ok && next_ok && ipv6_candidate_is_flagged(candidate) {
                    return true;
                }
            }
        } else {
            i += 1;
        }
    }
    false
}

impl Scanner<'_> {
    pub(crate) fn check_no_hardcoded_ip(&mut self, it: &StringLiteral<'_>) {
        if !self.options.has_rule(RULE_NAME) {
            return;
        }
        let value = it.value.as_str();
        if contains_flagged_ipv4(value) || contains_flagged_ipv6(value) {
            self.report(RULE_NAME, "hardcodedIp", it.span);
        }
    }
}
