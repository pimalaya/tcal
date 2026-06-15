//! Date conversions between calcard's [`PartialDateTime`], native TOML
//! date-times, and iCalendar digit forms. Projection emits a native TOML
//! `date`/`datetime`; apply reads one back, and still accepts the older
//! friendly `YYYY-MM-DD[ HH:MM[:SS]][ UTC]` string form.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
};

use calcard::common::PartialDateTime;
use toml_edit::{Date, Datetime, Offset, Time};

/// Shared hint for the date keys: a concrete example native TOML date-time.
pub const DATE_HINT: &str = "2026-06-13T14:30:00";

/// Whether a calcard date-time carries an explicit UTC marker (a trailing
/// `Z`), which calcard encodes as a zero numeric offset.
pub fn is_utc(dt: &PartialDateTime) -> bool {
    matches!((dt.tz_hour, dt.tz_minute), (Some(0), Some(0)))
}

/// Build a native TOML value from a calcard date-time, or `None` when it is
/// partial (a yearless or year-only date) and so has no native TOML form.
/// An all-day value becomes a local date, a UTC value an offset date-time,
/// anything else a local date-time; a named zone is carried separately, not
/// folded into the value.
pub fn toml_date(dt: &PartialDateTime) -> Option<Datetime> {
    let date = Date {
        year: dt.year?,
        month: dt.month?,
        day: dt.day?,
    };

    let Some((hour, minute)) = dt.hour.zip(dt.minute) else {
        return Some(Datetime {
            date: Some(date),
            time: None,
            offset: None,
        });
    };

    let time = Time {
        hour,
        minute,
        second: Some(dt.second.unwrap_or(0)),
        nanosecond: None,
    };

    Some(Datetime {
        date: Some(date),
        time: Some(time),
        offset: is_utc(dt).then_some(Offset::Z),
    })
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

/// Parse an `RRULE` `UNTIL` digit value (`20261231T235900Z`) into a native
/// TOML date-time, or `None` when it is not in the expected digit form.
pub fn until_to_toml(raw: &str) -> Option<Datetime> {
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

/// Render a calcard date-time as its RFC 5545 basic ISO 8601 string,
/// covering the partial forms native TOML cannot hold (a yearless `--0415`
/// or year-only `2009` date), with a `T..` time when present. This is the
/// projection fallback for the partial values [`toml_date`] returns `None`
/// for.
pub fn ical_date(dt: &PartialDateTime) -> String {
    let mut out = String::new();

    match (dt.year, dt.month, dt.day) {
        (Some(y), Some(m), Some(d)) => out.push_str(&format!("{y:04}{m:02}{d:02}")),
        (Some(y), Some(m), None) => out.push_str(&format!("{y:04}-{m:02}")),
        (Some(y), None, None) => out.push_str(&format!("{y:04}")),
        (None, Some(m), Some(d)) => out.push_str(&format!("--{m:02}{d:02}")),
        (None, Some(m), None) => out.push_str(&format!("--{m:02}")),
        (None, None, Some(d)) => out.push_str(&format!("---{d:02}")),
        _ => {}
    }

    if let Some(hour) = dt.hour {
        out.push_str(&format!("T{hour:02}"));
        if let Some(minute) = dt.minute {
            out.push_str(&format!("{minute:02}"));
            if let Some(second) = dt.second {
                out.push_str(&format!("{second:02}"));
            }
        }
    }

    out
}

/// Render a calcard UTC-offset (`TZOFFSETFROM`/`TZOFFSETTO`) as `±HHMM`,
/// mirroring calcard's writer; empty when the offset is absent.
pub fn offset_text(dt: &PartialDateTime) -> String {
    let (Some(hour), Some(minute)) = (dt.tz_hour, dt.tz_minute) else {
        return String::new();
    };
    let sign = if dt.tz_minus { '-' } else { '+' };

    format!("{sign}{hour:02}{minute:02}")
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
