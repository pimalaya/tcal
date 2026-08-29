//! # Value helpers
//!
//! The small conversions projection and apply share: rendering TOML scalars,
//! and reading a content line's value and parameters.
//!
//! A line is read through its logical form rather than through the syntax
//! tree's own split, because RFC 5545 section 3.1 ends the head at the first
//! colon outside a quoted parameter value and the tree's split is not quoted.
//! The same [`crate::template::patch`] grammar reads a line here and patches
//! it there, so what the projection shows and what a fold-back writes agree.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec::Vec,
};

use ical::tree::line::IcalLine;
use toml_edit::{Array, Item, TableLike, Value};

use crate::{
    ical::logical,
    template::patch::{head, split, value_of},
};

/// Render a string as a quoted, escaped TOML scalar.
pub fn toml_str(value: &str) -> String {
    Value::from(value).to_string().trim().to_string()
}

/// Render an integer string as a bare TOML number, `""` when blank (which
/// the caller ignores), or a quoted fallback when not a plain integer.
pub fn toml_number(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_owned()
    } else if value.parse::<i64>().is_ok() {
        value.to_owned()
    } else {
        toml_str(value)
    }
}

/// Render strings as a TOML array.
pub fn toml_array<S: AsRef<str>>(items: &[S]) -> String {
    let mut array = Array::new();

    for item in items {
        array.push(item.as_ref());
    }

    array.to_string().trim().to_string()
}

/// Render integers as a TOML array.
pub fn toml_int_array(items: &[i64]) -> String {
    let mut array = Array::new();

    for item in items {
        array.push(*item);
    }

    array.to_string().trim().to_string()
}

/// Escape an iCalendar text value per RFC 5545 section 3.3.11.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ',' => out.push_str("\\,"),
            ';' => out.push_str("\\;"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }

    out
}

/// Undo that escaping, the inverse of [`escape`]: what a calendar wrote as
/// `\,` is a comma, and either spelling of an escaped newline is one.
pub fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(next) => out.push(next),
            None => out.push('\\'),
        }
    }

    out
}

/// Append `;NAME=value` to `line` when the table entry is non-empty,
/// quoting on a parameter delimiter. `upper` uppercases closed vocabularies
/// (`ROLE`, `PARTSTAT`).
pub fn push_param(line: &mut String, name: &str, item: Option<&Item>, upper: bool) {
    let Some(value) = item
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    let value = if upper {
        value.to_uppercase()
    } else {
        value.to_owned()
    };

    line.push(';');
    line.push_str(name);
    line.push('=');

    if value.contains([',', ';', ':', '"']) {
        line.push('"');
        line.push_str(&value.replace('"', ""));
        line.push('"');
    } else {
        line.push_str(&value);
    }
}

/// The TOML tables addressed by an array-of-tables (`[[key]]`) or an inline
/// array of inline tables.
pub fn tables(item: &Item) -> Vec<&dyn TableLike> {
    if let Some(array) = item.as_array_of_tables() {
        array.iter().map(|table| table as &dyn TableLike).collect()
    } else if let Some(array) = item.as_array() {
        array
            .iter()
            .filter_map(|value| value.as_inline_table())
            .map(|table| table as &dyn TableLike)
            .collect()
    } else {
        Vec::new()
    }
}

/// The value a line carries, still escaped: everything after the colon that
/// ends its name and parameters, one inside a quoted parameter value not
/// counting.
///
/// This is the form a structured value (a recurrence rule) is read in, its
/// own separators being its syntax rather than the calendar's.
pub fn raw(line: &IcalLine<'_>) -> String {
    value_of(&logical(line)).to_owned()
}

/// A line's value as one unescaped string, its commas kept literal.
///
/// A single-valued property is one value however it is punctuated, so a comma
/// inside a URI or an unescaped one inside a summary stays in the value rather
/// than truncating it.
pub fn text(line: &IcalLine<'_>) -> String {
    unescape(&raw(line))
}

/// A line's value as its comma-separated items, each unescaped on its own.
pub fn items(line: &IcalLine<'_>) -> Vec<String> {
    let raw = raw(line);

    split_items(&raw).into_iter().map(unescape).collect()
}

/// The first value of a named parameter, the quotes it may be written with
/// stripped (RFC 5545 section 3.2).
pub fn param(line: &IcalLine<'_>, name: &str) -> Option<String> {
    let logical = logical(line);

    split(head(&logical), ';')
        .into_iter()
        .skip(1)
        .find_map(|param| {
            let (held, value) = param.split_once('=')?;

            held.eq_ignore_ascii_case(name)
                .then(|| value.trim_matches('"').to_owned())
        })
}

/// Split a value on its unescaped commas, an escaped one staying inside the
/// item it belongs to.
fn split_items(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut escaped = false;

    for (at, ch) in value.char_indices() {
        match ch {
            '\\' if !escaped => escaped = true,
            ',' if !escaped => {
                out.push(&value[start..at]);
                start = at + 1;
            }
            _ => escaped = false,
        }
    }

    out.push(&value[start..]);
    out
}

/// A calendar address without its `mailto:` scheme (any case), for display.
pub fn strip_mailto(value: &str) -> &str {
    match value.get(..7) {
        Some(scheme) if scheme.eq_ignore_ascii_case("mailto:") => &value[7..],
        _ => value,
    }
}

/// A calendar address with a scheme: a bare address gains `mailto:`.
pub fn ensure_mailto(value: &str) -> String {
    if value.contains(':') {
        value.to_owned()
    } else {
        format!("mailto:{value}")
    }
}

/// A non-empty string from a TOML table key.
pub fn table_text(table: &dyn TableLike, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// An integer from a TOML table key, accepting a bare number or a numeric
/// string.
pub fn table_int(table: &dyn TableLike, key: &str) -> Option<i64> {
    let item = table.get(key)?;
    item.as_integer()
        .or_else(|| item.as_str().and_then(|value| value.trim().parse().ok()))
}
