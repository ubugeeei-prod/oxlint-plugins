//! Source-text utilities ported from upstream `src/utils.ts`.
//!
//! The whole parser pipeline operates in **UTF-16 code-unit space**, exactly
//! like the upstream TypeScript parser (which indexes JS strings via
//! `charCodeAt`). ESLint reports `loc.column` as a UTF-16 column and the test
//! fixtures encode those columns, so reproducing them faithfully requires the
//! same unit. libpg_query reports node `location` as a UTF-8 byte offset, so
//! [`Source::byte_to_unit`] converts those to UTF-16 offsets before any lookup.
#![allow(
    clippy::disallowed_types,
    clippy::disallowed_methods,
    clippy::disallowed_macros,
    reason = "serde_json / libpg_query interop boundary: this parser layer mirrors upstream's JS string semantics and works with owned String/Vec. The carton-collection policy governs rule hot-path state, not this boundary."
)]

/// 1-indexed line, 0-indexed UTF-16 column — ESLint's `loc` convention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub line: u32,
    pub column: u32,
}

/// The source text as UTF-16 code units plus the line- and byte-offset maps.
pub struct Source {
    /// The source as UTF-16 code units.
    units: Vec<u16>,
    /// UTF-16 offset at which each line starts (`line_starts[0] == 0`).
    line_starts: Vec<u32>,
    /// `byte_to_unit[b]` is the UTF-16 offset for UTF-8 byte offset `b`.
    /// `None` when the source is pure ASCII (offsets are then identity).
    byte_to_unit: Option<Vec<u32>>,
}

impl Source {
    pub fn new(code: &str) -> Self {
        let units: Vec<u16> = code.encode_utf16().collect();

        let mut line_starts = vec![0u32];
        for (index, unit) in units.iter().enumerate() {
            if *unit == u16::from(b'\n') {
                line_starts.push((index + 1) as u32);
            }
        }

        let byte_to_unit = if code.is_ascii() {
            None
        } else {
            // Mirror `createByteToCharOffset`: map every UTF-8 byte offset to
            // the UTF-16 unit offset of the char that owns it.
            let mut map = Vec::with_capacity(code.len() + 1);
            let mut unit_offset: u32 = 0;
            for ch in code.chars() {
                let utf8_len = ch.len_utf8();
                for _ in 0..utf8_len {
                    map.push(unit_offset);
                }
                unit_offset += ch.len_utf16() as u32;
            }
            map.push(unit_offset);
            Some(map)
        };

        Self {
            units,
            line_starts,
            byte_to_unit,
        }
    }

    /// Total length in UTF-16 code units.
    pub fn len(&self) -> u32 {
        self.units.len() as u32
    }

    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }

    /// The UTF-16 code unit at `offset`, if in range.
    pub fn unit_at(&self, offset: u32) -> Option<u16> {
        self.units.get(offset as usize).copied()
    }

    /// The ASCII byte at `offset`, if the unit there is ASCII.
    pub fn ascii_at(&self, offset: u32) -> Option<u8> {
        match self.unit_at(offset) {
            Some(u) if u < 0x80 => Some(u as u8),
            _ => None,
        }
    }

    /// Decode the UTF-16 slice `[start, end)` back into a `String`.
    pub fn slice(&self, start: u32, end: u32) -> String {
        let start = start.min(self.len()) as usize;
        let end = end.min(self.len()) as usize;
        if start >= end {
            return String::new();
        }
        String::from_utf16_lossy(&self.units[start..end])
    }

    /// Convert a libpg_query UTF-8 byte offset to a UTF-16 unit offset.
    pub fn byte_to_unit(&self, byte_offset: i64) -> Option<u32> {
        if byte_offset < 0 {
            return None;
        }
        match &self.byte_to_unit {
            // Pure-ASCII fast path: byte offset == unit offset.
            None => Some(byte_offset as u32),
            Some(map) => {
                let idx = byte_offset as usize;
                Some(map.get(idx).copied().unwrap_or(self.len()))
            }
        }
    }

    /// Resolve a UTF-16 offset to a 1-indexed line / 0-indexed column.
    pub fn position(&self, offset: u32) -> Position {
        // Largest line index whose start offset is <= `offset`.
        let mut left = 0usize;
        let mut right = self.line_starts.len() - 1;
        while left < right {
            let mid = (left + right).div_ceil(2);
            if self.line_starts[mid] <= offset {
                left = mid;
            } else {
                right = mid - 1;
            }
        }
        let line_start = self.line_starts[left];
        Position {
            line: (left + 1) as u32,
            column: offset.saturating_sub(line_start),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Source;

    #[test]
    fn ascii_positions() {
        let s = Source::new("SELECT 1\nFROM t");
        assert_eq!(s.position(0).line, 1);
        assert_eq!(s.position(0).column, 0);
        let p = s.position(9); // 'F' of FROM starts line 2
        assert_eq!(p.line, 2);
        assert_eq!(p.column, 0);
        assert_eq!(s.position(10).column, 1); // 'R'
        assert_eq!(s.byte_to_unit(9), Some(9));
    }

    #[test]
    fn non_ascii_byte_to_unit() {
        // "é" is 2 UTF-8 bytes, 1 UTF-16 unit.
        let s = Source::new("'é' a");
        // byte 0 -> unit 0 ('), byte 1 starts 'é' -> unit 1, bytes 1..3 map to unit 1
        assert_eq!(s.byte_to_unit(0), Some(0));
        assert_eq!(s.byte_to_unit(1), Some(1));
        assert_eq!(s.byte_to_unit(3), Some(2)); // closing quote at unit 2
    }

    // The following mirror upstream `src/utils.test.ts` (`createLineMap` /
    // `createByteToCharOffset`) so position resolution and byte↔unit conversion
    // track upstream exactly. ESLint columns are UTF-16 code units. Three
    // upstream cases are intentionally not ported:
    //   * the negative-offset case — not representable in this `u32` API (and
    //     offsets are never negative in practice);
    //   * the `lineMap.code` property — `Source` stores UTF-16 units, not the
    //     original string, so there is no equivalent field;
    //   * the large-document "performance comparison" — redundant correctness
    //     coverage already exercised by the position tests below.
    fn pos(code: &str, offset: u32) -> (u32, u32) {
        let p = Source::new(code).position(offset);
        (p.line, p.column)
    }

    #[test]
    fn get_position_single_and_multi_line() {
        assert_eq!(pos("SELECT * FROM users", 0), (1, 0));
        assert_eq!(pos("SELECT * FROM users", 7), (1, 7));
        assert_eq!(pos("SELECT * FROM users", 19), (1, 19)); // end of line
        assert_eq!(pos("SELECT *\nFROM users", 9), (2, 0)); // second line start
        assert_eq!(pos("SELECT *\nFROM users", 14), (2, 5)); // second line middle
        assert_eq!(pos("SELECT *\nFROM users\nWHERE id = 1", 21), (3, 1));
    }

    #[test]
    fn get_position_edge_cases() {
        assert_eq!(pos("", 0), (1, 0)); // empty string
        assert_eq!(pos("\n\n\n", 2), (3, 0)); // only newlines
        assert_eq!(pos("SELECT *\r\nFROM users", 10), (2, 0)); // CRLF
        assert_eq!(pos("SELECT * FROM users", 29), (1, 29)); // beyond length
        assert_eq!(pos("SELECT * FROM users\n", 20), (2, 0)); // trailing newline
        assert_eq!(pos("SELECT *\nFROM users\r\nWHERE id = 1\n", 21), (3, 0)); // mixed
    }

    #[test]
    fn line_start_offsets() {
        assert_eq!(Source::new("SELECT * FROM users").line_starts, vec![0]);
        assert_eq!(
            Source::new("SELECT *\nFROM users\nWHERE id = 1").line_starts,
            vec![0, 9, 20]
        );
        assert_eq!(
            Source::new("SELECT *\n\nFROM users").line_starts,
            vec![0, 9, 10]
        );
        assert_eq!(
            Source::new("SELECT *\n\n\nFROM users").line_starts,
            vec![0, 9, 10, 11]
        );
    }

    #[test]
    fn byte_to_unit_identity_and_multibyte() {
        let ascii = Source::new("SELECT * FROM users");
        assert_eq!(ascii.byte_to_unit(0), Some(0));
        assert_eq!(ascii.byte_to_unit(7), Some(7));
        assert_eq!(ascii.byte_to_unit(19), Some(19));

        // "-- 日本語\nSELECT 1": each CJK char is 3 UTF-8 bytes, 1 UTF-16 unit.
        let s = Source::new("-- 日本語\nSELECT 1");
        assert_eq!(s.byte_to_unit(20), Some(14)); // the `1`
        assert_eq!(s.byte_to_unit(13), Some(7)); // the `S`
    }

    #[test]
    fn byte_to_unit_surrogate_pairs_and_clamping() {
        // "😀" is 4 UTF-8 bytes and 2 UTF-16 units (a surrogate pair).
        let s = Source::new("-- 😀\nSELECT 1");
        assert_eq!(s.byte_to_unit(15), Some(13)); // the `1`
        assert_eq!(s.byte_to_unit(3), Some(3)); // first byte of the emoji
        // Byte offsets past the end clamp to the UTF-16 length.
        let cjk = Source::new("日本語");
        assert_eq!(cjk.byte_to_unit(100), Some(cjk.len()));
    }
}
