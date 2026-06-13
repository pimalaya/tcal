//! Durations (`DURATION`, an alarm `TRIGGER` offset) as dotted
//! `<key>.{week,day,hour,min,sec}` magnitude keys, the sign implied.

use calcard::icalendar::ICalendarEntry;
use toml_edit::TableLike;

use crate::template::{
    line::{Line, int_line},
    util::{scalar_text, table_int, table_text, toml_str},
};

/// Project a duration entry as dotted magnitude keys, the field's hint on
/// the leading line. The sign is implied by context, so the parts are
/// unsigned; a value that is not a plain duration (an absolute date-time
/// trigger) is shown raw as `<prefix>.raw`, round-tripping intact.
pub fn duration_lines(
    entry: Option<&ICalendarEntry>,
    prefix: &str,
    hint: Option<&str>,
) -> Vec<Line> {
    let value = entry.map(scalar_text).filter(|value| !value.is_empty());
    let parts = value.as_deref().and_then(parse_duration);

    if let Some(value) = &value
        && parts.is_none()
    {
        return vec![Line {
            lhs: format!("{prefix}.raw = {}", toml_str(value)),
            hint: Some("raw duration; tcal could not break it into parts".to_owned()),
        }];
    }

    let (week, day, hour, minute, second) = parts.unwrap_or_default();
    let set = |value: i64| (value != 0).then_some(value);

    vec![
        int_line(&format!("{prefix}.week"), set(week), hint),
        int_line(&format!("{prefix}.day"), set(day), None),
        int_line(&format!("{prefix}.hour"), set(hour), None),
        int_line(&format!("{prefix}.min"), set(minute), None),
        int_line(&format!("{prefix}.sec"), set(second), None),
    ]
}

/// Assemble an iCalendar duration from a table's `week/day/hour/min/sec`
/// parts (or a raw `raw` escape hatch), prefixing `-` when `negative`. A
/// lone week stays `P<n>W`; weeks otherwise fold into days. `None` when no
/// part is set.
pub fn duration_value(table: &dyn TableLike, negative: bool) -> Option<String> {
    if let Some(raw) = table_text(table, "raw") {
        return Some(raw);
    }

    let week = table_int(table, "week").unwrap_or(0);
    let day = table_int(table, "day").unwrap_or(0);
    let hour = table_int(table, "hour").unwrap_or(0);
    let minute = table_int(table, "min").unwrap_or(0);
    let second = table_int(table, "sec").unwrap_or(0);

    if [week, day, hour, minute, second]
        .iter()
        .all(|part| *part == 0)
    {
        return None;
    }

    let sign = if negative { "-" } else { "" };

    if week != 0 && [day, hour, minute, second].iter().all(|part| *part == 0) {
        return Some(format!("{sign}P{week}W"));
    }

    let days = day + 7 * week;
    let mut out = format!("{sign}P");

    if days != 0 {
        out.push_str(&days.to_string());
        out.push('D');
    }
    if hour != 0 || minute != 0 || second != 0 {
        out.push('T');
        if hour != 0 {
            out.push_str(&hour.to_string());
            out.push('H');
        }
        if minute != 0 {
            out.push_str(&minute.to_string());
            out.push('M');
        }
        if second != 0 {
            out.push_str(&second.to_string());
            out.push('S');
        }
    }

    Some(out)
}

/// Parse an iCalendar duration (`P1DT2H30M`, `PT15M`, `P2W`, optionally
/// signed) into unsigned magnitudes; `None` when not a duration.
fn parse_duration(value: &str) -> Option<(i64, i64, i64, i64, i64)> {
    let value = value.trim();
    let value = value.strip_prefix(['+', '-']).unwrap_or(value);
    let body = value.strip_prefix('P')?;

    let (date, time) = match body.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (body, None),
    };

    let (mut week, mut day, mut hour, mut minute, mut second) = (0, 0, 0, 0, 0);

    for (number, unit) in scan_units(date)? {
        match unit {
            'W' => week = number,
            'D' => day = number,
            _ => return None,
        }
    }

    if let Some(time) = time {
        let units = scan_units(time)?;
        if units.is_empty() {
            return None;
        }
        for (number, unit) in units {
            match unit {
                'H' => hour = number,
                'M' => minute = number,
                'S' => second = number,
                _ => return None,
            }
        }
    }

    if [week, day, hour, minute, second]
        .iter()
        .all(|part| *part == 0)
    {
        return None;
    }

    Some((week, day, hour, minute, second))
}

/// Split a duration segment (`1D`, `2H30M`) into `(number, unit)` pairs;
/// `None` when a number has no unit or a unit has no number.
fn scan_units(segment: &str) -> Option<Vec<(i64, char)>> {
    let mut pairs = Vec::new();
    let mut digits = String::new();

    for ch in segment.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            if digits.is_empty() {
                return None;
            }
            pairs.push((digits.parse().ok()?, ch));
            digits.clear();
        }
    }

    if digits.is_empty() { Some(pairs) } else { None }
}
