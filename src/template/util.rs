//! Small value helpers shared across projection and apply: TOML rendering,
//! iCalendar text escaping, and reading calcard entry values.

use calcard::icalendar::{ICalendarEntry, ICalendarParameterName};
use toml_edit::{Array, Item, TableLike, Value};

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

/// First value of an entry as text, falling back to its owned text form for
/// typed values (integers, durations, recurrence rules, ...).
pub fn scalar_text(entry: &ICalendarEntry) -> String {
    if let Some(text) = entry.values.first().and_then(|value| value.as_text()) {
        return text.to_owned();
    }

    entry
        .values
        .first()
        .cloned()
        .and_then(|value| value.into_text())
        .map(|text| text.into_owned())
        .unwrap_or_default()
}

/// First value of an entry as borrowed text.
pub fn entry_text(entry: &ICalendarEntry) -> Option<&str> {
    entry.values.first().and_then(|value| value.as_text())
}

/// First value of a named parameter as owned text.
pub fn param(entry: &ICalendarEntry, name: &ICalendarParameterName) -> Option<String> {
    entry
        .parameters(name)
        .next()
        .and_then(|value| value.as_text())
        .map(str::to_owned)
}

/// A calendar address without its `mailto:` scheme, for display.
pub fn strip_mailto(value: &str) -> &str {
    value
        .strip_prefix("mailto:")
        .or_else(|| value.strip_prefix("MAILTO:"))
        .unwrap_or(value)
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
