//! # TOML side
//!
//! Rendering a value as the TOML the document writes, and reading one back out
//! of an edited table.
//!
//! A line is read through the [`crate::template::patch`] grammar a fold-back
//! patches it with, so what the projection shows and what it writes agree.

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};

use toml_edit::{Array, Item, TableLike, Value};
/// Render a string as a quoted, escaped TOML scalar.
pub fn toml_str(value: &str) -> String {
    Value::from(value).to_string().trim().to_string()
}

/// Render an integer string as a bare TOML number.
///
/// `""` when blank, which the caller ignores, and a quoted fallback when
/// the value is not a plain integer.
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

/// Append `;NAME=value` to `line` when the table entry is non-empty.
///
/// The value is quoted on a parameter delimiter, and `upper` uppercases the
/// closed vocabularies (`ROLE`, `PARTSTAT`).
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

/// The TOML tables addressed by an array-of-tables (`[[key]]`).
///
/// An inline array of inline tables is read the same way.
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

/// A non-empty string from a TOML table key.
pub fn table_text(table: &dyn TableLike, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// An integer from a TOML table key.
///
/// A bare number and a numeric string are both accepted.
pub fn table_int(table: &dyn TableLike, key: &str) -> Option<i64> {
    let item = table.get(key)?;
    item.as_integer()
        .or_else(|| item.as_str().and_then(|value| value.trim().parse().ok()))
}
