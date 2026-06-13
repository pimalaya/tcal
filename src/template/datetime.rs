//! Friendly `YYYY-MM-DD[ HH:MM[:SS]][ UTC]` date-times to and from
//! iCalendar digit forms.

use calcard::common::PartialDateTime;

/// Shared hint for the friendly date keys: a concrete example date-time.
pub const DATE_HINT: &str = "2026-06-13 14:30";

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

/// Render a calcard date-time as a friendly `YYYY-MM-DD[ HH:MM[:SS]]`,
/// appending ` UTC` for a UTC value; a value with no time is an all-day
/// `YYYY-MM-DD`.
pub fn friendly_date(dt: &PartialDateTime) -> String {
    let (Some(year), Some(month), Some(day)) = (dt.year, dt.month, dt.day) else {
        return String::new();
    };
    let date = format!("{year:04}-{month:02}-{day:02}");

    let (Some(hour), Some(minute)) = (dt.hour, dt.minute) else {
        return date;
    };
    let mut out = format!("{date} {hour:02}:{minute:02}");

    if let Some(second) = dt.second.filter(|second| *second != 0) {
        out.push_str(&format!(":{second:02}"));
    }
    if matches!((dt.tz_hour, dt.tz_minute), (Some(0), Some(0))) {
        out.push_str(" UTC");
    }

    out
}

/// Render an `RRULE` `UNTIL` value (`20261231T000000Z`) as a friendly
/// `YYYY-MM-DD [HH:MM[:SS]] [UTC]`, passing it through verbatim when it is
/// not in the expected digit form.
pub fn ical_to_friendly(raw: &str) -> String {
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
        return raw.to_owned();
    }
    let mut out = format!("{}-{}-{}", &date[0..4], &date[4..6], &date[6..8]);

    if let Some(time) =
        time.filter(|time| time.len() >= 4 && time.bytes().all(|byte| byte.is_ascii_digit()))
    {
        out.push_str(&format!(" {}:{}", &time[0..2], &time[2..4]));
        if time.len() >= 6 && &time[4..6] != "00" {
            out.push_str(&format!(":{}", &time[4..6]));
        }
        if utc {
            out.push_str(" UTC");
        }
    }

    out
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
