//! Source-location helpers shared by directive-comment rules.

/// A source position. `line` is 1-indexed; `column` is 0-indexed. A `column`
/// of `-1` is a sentinel meaning "force the whole line" (matching upstream
/// `toForceLocation`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Position {
    pub line: u32,
    pub column: i32,
}

/// A source range.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Location {
    pub start: Position,
    pub end: Position,
}

/// Build a location that ignores the start column, used so diagnostics about a
/// directive comment sort to the start of the line. Mirrors upstream
/// `toForceLocation`.
pub fn to_force_location(location: Location) -> Location {
    Location {
        start: Position {
            line: location.start.line,
            column: -1,
        },
        end: location.end,
    }
}

/// `true` when `a` is at or before `b`. Mirrors upstream `lte`.
pub fn lte(a: Position, b: Position) -> bool {
    a.line < b.line || (a.line == b.line && a.column <= b.column)
}
