//! # Dates
//!
//! Conversions between iCalendar digit forms and native TOML date-times.
//!
//! The projection emits a native TOML `date` or `datetime` read from the
//! value as the calendar writes it; apply reads one back, and still accepts
//! the older friendly `YYYY-MM-DD[ HH:MM[:SS]][ UTC]` string form.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
};

use toml_edit::{Date, Datetime, Offset, Time};

/// Shared hint for the date keys: a concrete example native TOML date-time.
pub const DATE_HINT: &str = "2026-06-13T14:30:00";

/// Whether a date-time is written as UTC, which iCalendar marks with a
/// trailing `Z`.
pub fn is_utc(dtm: &Datetime) -> bool {
    matches!(dtm.offset, Some(Offset::Z))
}

/// Build an iCalendar date line from a native TOML date-time and optional
/// named zone: a bare date becomes a `VALUE=DATE` property, a UTC value
/// keeps its `Z`, and a named zone becomes a `TZID` parameter. A numeric
/// offset other than `Z` is treated as floating, as iCalendar has no
/// offset date-time form.
pub fn toml_date_line(name: &str, dtm: &Datetime, tz: Option<&str>) -> String {
    let Some(date) = dtm.date else {
        return format!("{name}:{dtm}");
    };
    let date = format!("{:04}{:02}{:02}", date.year, date.month, date.day);

    let Some(time) = dtm.time else {
        return format!("{name};VALUE=DATE:{date}");
    };
    let time = format!(
        "{:02}{:02}{:02}",
        time.hour,
        time.minute,
        time.second.unwrap_or(0)
    );

    match dtm.offset {
        Some(Offset::Z) => format!("{name}:{date}T{time}Z"),
        _ => match tz {
            Some(zone) => format!("{name};TZID={zone}:{date}T{time}"),
            None => format!("{name}:{date}T{time}"),
        },
    }
}

/// Read an iCalendar date or date-time value (`20261231T235900Z`) as a
/// native TOML one.
///
/// `None` for anything not in that digit form, which the projection then
/// shows as the string the calendar wrote, so it round-trips whole.
pub fn toml_date(raw: &str) -> Option<Datetime> {
    let raw = raw.trim();
    let (body, utc) = match raw.strip_suffix('Z') {
        Some(body) => (body, true),
        None => (raw, false),
    };
    let (date, time) = match body.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None => (body, None),
    };

    if date.len() != 8 || !date.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let date = Date {
        year: date[0..4].parse().ok()?,
        month: date[4..6].parse().ok()?,
        day: date[6..8].parse().ok()?,
    };

    let Some(time) = time else {
        return Some(Datetime {
            date: Some(date),
            time: None,
            offset: None,
        });
    };
    if time.len() < 6 || !time.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let time = Time {
        hour: time[0..2].parse().ok()?,
        minute: time[2..4].parse().ok()?,
        second: Some(time[4..6].parse().ok()?),
        nanosecond: None,
    };

    Some(Datetime {
        date: Some(date),
        time: Some(time),
        offset: utc.then_some(Offset::Z),
    })
}

/// Render a native TOML date-time back to an `RRULE` `UNTIL` digit value.
pub fn until_to_ical(dtm: &Datetime) -> String {
    let Some(date) = dtm.date else {
        return dtm.to_string();
    };
    let mut out = format!("{:04}{:02}{:02}", date.year, date.month, date.day);

    if let Some(time) = dtm.time {
        out.push_str(&format!(
            "T{:02}{:02}{:02}",
            time.hour,
            time.minute,
            time.second.unwrap_or(0)
        ));
        if matches!(dtm.offset, Some(Offset::Z)) {
            out.push('Z');
        }
    }

    out
}

/// Build an iCalendar date line from a friendly value and optional time
/// zone, passing the value verbatim when it is not in the friendly form.
pub fn date_line(name: &str, value: &str, tz: Option<&str>) -> String {
    match parse_friendly_date(value) {
        Some((date, None, _)) => format!("{name};VALUE=DATE:{date}"),
        Some((date, Some(time), true)) => format!("{name}:{date}T{time}Z"),
        Some((date, Some(time), false)) => match tz {
            Some(zone) => format!("{name};TZID={zone}:{date}T{time}"),
            None => format!("{name}:{date}T{time}"),
        },
        None => match tz {
            Some(zone) => format!("{name};TZID={zone}:{value}"),
            None => format!("{name}:{value}"),
        },
    }
}

/// Parse a friendly date-time into its iCalendar digit parts: the date
/// (`YYYYMMDD`), an optional time (`HHMMSS`, `None` for an all-day date),
/// and whether it is UTC.
pub fn parse_friendly_date(value: &str) -> Option<(String, Option<String>, bool)> {
    let value = value.trim();
    let (rest, utc) = match value
        .strip_suffix(" UTC")
        .or_else(|| value.strip_suffix(" utc"))
    {
        Some(rest) => (rest.trim_end(), true),
        None => (value, false),
    };

    let mut parts = rest.split_whitespace();
    let date = parts.next()?;
    let time = parts.next();
    if parts.next().is_some() {
        return None;
    }

    let mut ymd = date.split('-');
    let year: u16 = ymd.next()?.parse().ok()?;
    let month: u8 = ymd.next()?.parse().ok()?;
    let day: u8 = ymd.next()?.parse().ok()?;
    if ymd.next().is_some() {
        return None;
    }
    let date = format!("{year:04}{month:02}{day:02}");

    let time = match time {
        None => None,
        Some(time) => {
            let mut hms = time.split(':');
            let hour: u8 = hms.next()?.parse().ok()?;
            let minute: u8 = hms.next()?.parse().ok()?;
            let second: u8 = match hms.next() {
                Some(second) => second.parse().ok()?,
                None => 0,
            };
            if hms.next().is_some() {
                return None;
            }
            Some(format!("{hour:02}{minute:02}{second:02}"))
        }
    };

    Some((date, time, utc))
}

/// Convert a friendly date back to the `RRULE` `UNTIL` digit form, passing
/// it through verbatim when it is not friendly.
pub fn friendly_to_ical(value: &str) -> String {
    match parse_friendly_date(value) {
        Some((date, None, _)) => date,
        Some((date, Some(time), true)) => format!("{date}T{time}Z"),
        Some((date, Some(time), false)) => format!("{date}T{time}"),
        None => value.to_owned(),
    }
}
