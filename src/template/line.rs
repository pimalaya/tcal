//! # Lines
//!
//! Projected lines and their tab-aligned inline comments.

use alloc::{borrow::ToOwned, format, string::String};

/// Tab width assumed when aligning comments; their column is a multiple.
const TAB_WIDTH: usize = 8;

/// A projected line: a left side and an optional inline hint.
pub struct Line {
    /// The line's text, a `key = value` pair, empty for a group separator.
    pub lhs: String,
    /// The `#` comment shown after the left side, where the field has one.
    pub hint: Option<String>,
}

/// A dotted integer key, a recurrence or a duration part.
///
/// A bare number when set, an empty string otherwise, which apply ignores.
pub fn int_line(key: &str, value: Option<i64>, hint: Option<&str>) -> Line {
    let lhs = match value {
        Some(value) => format!("{key} = {value}"),
        None => format!("{key} = \"\""),
    };
    Line {
        lhs,
        hint: hint.map(str::to_owned),
    }
}

/// The shared column a component's inline `#` comments align on.
///
/// The first tab stop past the widest hinted left side, so every hinted
/// line reaches it with at least one tab: one tab too many is harmless,
/// one short would break the column.
pub fn comment_column<'a>(lines: impl Iterator<Item = &'a Line>) -> usize {
    let widest = lines
        .filter(|line| line.hint.is_some())
        .map(|line| line.lhs.len())
        .max()
        .unwrap_or(0);

    (widest / TAB_WIDTH + 1) * TAB_WIDTH
}

/// Emit lines, padding a hinted line with tabs so its `#` lands on `column`.
///
/// A line with an empty left side is a blank group separator.
pub fn emit_lines(out: &mut String, lines: &[Line], column: usize) {
    for line in lines {
        out.push_str(&line.lhs);

        if let Some(hint) = &line.hint {
            let mut at = line.lhs.len();
            while at < column {
                out.push('\t');
                at = (at / TAB_WIDTH + 1) * TAB_WIDTH;
            }
            out.push_str("# ");
            out.push_str(hint);
        }

        out.push('\n');
    }
}
