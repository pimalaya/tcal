//! Recurrence rules (`RRULE`) as dotted `<prefix>.*` keys of friendly parts,
//! with a raw escape hatch for parts tcal does not model.

use calcard::icalendar::ICalendarEntry;
use toml_edit::TableLike;

use crate::template::{
    datetime::{friendly_to_ical, ical_to_friendly},
    line::{Line, int_line},
    util::{scalar_text, table_int, table_text, toml_array, toml_int_array, toml_str},
};

/// The `RRULE` tokens tcal models, in calcard's canonical serialization
/// order; a rule using any other token is shown raw to round-trip.
const RECUR_KEYS: &[&str] = &[
    "FREQ",
    "UNTIL",
    "COUNT",
    "INTERVAL",
    "BYDAY",
    "BYMONTHDAY",
    "BYMONTH",
    "BYSETPOS",
    "WKST",
];

/// Project a recurrence entry as dotted `<prefix>.*` keys of friendly parts.
/// A rule using a part tcal does not model is shown as a single raw
/// `<prefix>.rule` key instead.
pub fn recur_lines(entry: Option<&ICalendarEntry>, prefix: &str) -> Vec<Line> {
    let rule = entry.map(scalar_text).filter(|rule| !rule.is_empty());
    let parts = rule.as_deref().map(parse_rrule).unwrap_or_default();

    if let Some(rule) = &rule
        && !parts
            .iter()
            .all(|(name, _)| RECUR_KEYS.contains(&name.as_str()))
    {
        return vec![Line {
            lhs: format!("{prefix}.rule = {}", toml_str(rule)),
            hint: Some("raw RRULE; has parts tcal does not model".to_owned()),
        }];
    }

    let get = |key: &str| {
        parts
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    };
    let get_int = |key: &str| get(key).and_then(|value| value.trim().parse::<i64>().ok());

    vec![
        Line {
            lhs: format!(
                "{prefix}.frequency = {}",
                toml_str(&get("FREQ").unwrap_or_default().to_lowercase())
            ),
            hint: Some("secondly, minutely, hourly, daily, weekly, monthly, yearly".to_owned()),
        },
        int_line(
            &format!("{prefix}.interval"),
            get_int("INTERVAL"),
            Some("every N periods"),
        ),
        int_line(
            &format!("{prefix}.count"),
            get_int("COUNT"),
            Some("total occurrences; alternative to until"),
        ),
        Line {
            lhs: format!(
                "{prefix}.until = {}",
                toml_str(&get("UNTIL").map(ical_to_friendly).unwrap_or_default())
            ),
            hint: Some("2026-06-13 14:30".to_owned()),
        },
        str_list_line(
            &format!("{prefix}.by-day"),
            get("BYDAY"),
            "mo, tu, we, th, fr, sa, su; with an ordinal like -1su, 2mo",
        ),
        int_list_line(&format!("{prefix}.by-month"), get("BYMONTH"), "1 to 12"),
        int_list_line(
            &format!("{prefix}.by-month-day"),
            get("BYMONTHDAY"),
            "1 to 31, negative counts from the end (-1 = last)",
        ),
        int_list_line(
            &format!("{prefix}.by-position"),
            get("BYSETPOS"),
            "nth occurrence in the period; -1 = last",
        ),
        Line {
            lhs: format!(
                "{prefix}.week-start = {}",
                toml_str(&get("WKST").unwrap_or_default().to_lowercase())
            ),
            hint: Some("mo, tu, we, th, fr, sa, su".to_owned()),
        },
    ]
}

/// Assemble an `RRULE` value from a recurrence table, in [`RECUR_KEYS`]
/// order so an untouched rule round-trips byte-for-byte. A non-empty `rule`
/// key short-circuits to its raw value.
pub fn recur_rule(table: &dyn TableLike) -> Option<String> {
    if let Some(rule) = table_text(table, "rule") {
        return Some(rule);
    }

    let freq = table_text(table, "frequency")?;
    let mut parts = vec![format!("FREQ={}", freq.to_uppercase())];

    if let Some(until) = table_text(table, "until") {
        parts.push(format!("UNTIL={}", friendly_to_ical(&until)));
    }
    if let Some(count) = table_int(table, "count") {
        parts.push(format!("COUNT={count}"));
    }
    if let Some(interval) = table_int(table, "interval") {
        parts.push(format!("INTERVAL={interval}"));
    }

    let byday = str_list(table, "by-day");
    if !byday.is_empty() {
        parts.push(format!("BYDAY={}", byday.join(",")));
    }

    for (key, token) in [
        ("by-month-day", "BYMONTHDAY"),
        ("by-month", "BYMONTH"),
        ("by-position", "BYSETPOS"),
    ] {
        let values = int_list(table, key);
        if !values.is_empty() {
            parts.push(format!("{token}={}", join_ints(&values)));
        }
    }

    if let Some(wkst) = table_text(table, "week-start") {
        parts.push(format!("WKST={}", wkst.to_uppercase()));
    }

    Some(parts.join(";"))
}

/// Split an `RRULE` value into uppercased token names with their raw value.
fn parse_rrule(rule: &str) -> Vec<(String, String)> {
    rule.split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(name, value)| (name.to_uppercase(), value.to_owned()))
        .collect()
}

/// A recurrence string-list key (e.g. `by-day`), lowercased for display.
fn str_list_line(key: &str, value: Option<&str>, hint: &str) -> Line {
    let items: Vec<String> = value
        .map(|value| value.split(',').map(str::to_lowercase).collect())
        .unwrap_or_default();
    Line {
        lhs: format!("{key} = {}", toml_array(&items)),
        hint: Some(hint.to_owned()),
    }
}

/// A recurrence integer-list key (e.g. `by-month-day`).
fn int_list_line(key: &str, value: Option<&str>, hint: &str) -> Line {
    let items: Vec<i64> = value
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| item.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();
    Line {
        lhs: format!("{key} = {}", toml_int_array(&items)),
        hint: Some(hint.to_owned()),
    }
}

/// An uppercased string list from a recurrence table key.
fn str_list(table: &dyn TableLike, key: &str) -> Vec<String> {
    let Some(array) = table.get(key).and_then(|item| item.as_array()) else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_uppercase)
        .collect()
}

/// An integer list from a recurrence table key, accepting bare numbers or
/// numeric strings.
fn int_list(table: &dyn TableLike, key: &str) -> Vec<i64> {
    let Some(array) = table.get(key).and_then(|item| item.as_array()) else {
        return Vec::new();
    };

    array
        .iter()
        .filter_map(|value| {
            value
                .as_integer()
                .or_else(|| value.as_str().and_then(|value| value.trim().parse().ok()))
        })
        .collect()
}

/// Join integers on commas for an `RRULE` part.
fn join_ints(items: &[i64]) -> String {
    items
        .iter()
        .map(i64::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
