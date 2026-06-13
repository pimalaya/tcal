//! Projection between a calcard [`ICalendar`] and an ergonomic TOML
//! buffer.
//!
//! [`project`] turns the first `VEVENT` of an iCalendar into a fillable
//! TOML form: known fields are prefilled, the rest are listed empty (an
//! empty value means the same as a removed line, so nothing is commented
//! out). Cryptic date-times (`20260613T140000`) become a friendly
//! `2026-06-13 14:00`, and the time zone is broken out onto its own key.
//! [`apply`] takes the original iCalendar text plus the edited buffer and
//! produces an updated iCalendar, patching only the modeled `VEVENT`
//! lines the user changed (through the format-preserving [`crate::edit`])
//! while keeping every unmodeled property (custom `X-*`, ...) and every
//! other component (`VALARM`, `VTIMEZONE`, ...) byte-for-byte.
//!
//! `UID` and `DTSTAMP` are intentionally not modeled: they are managed
//! by the app (seeded for new events, preserved otherwise) and cannot be
//! set through the buffer.
//!
//! The buffer is an editing affordance, not an interchange format:
//! `apply` always needs the original iCalendar text, because that is
//! where unmodeled properties and sibling components live.
//!
//! NOTE: TOML attributes every bare key after a `[table]` / `[[array]]`
//! header to that table, so [`FIELDS`] lists all scalar/list keys first
//! and every sectioned property (`ATTENDEE`) last.

use std::fmt::Write as _;

use calcard::{
    common::PartialDateTime,
    icalendar::{ICalendar, ICalendarComponentType, ICalendarEntry, ICalendarParameterName},
};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::error::{Result, TcalError};

/// Project the first `VEVENT` of an iCalendar into a fillable TOML form.
///
/// An iCalendar with no `VEVENT` yields a blank template: the bare keys
/// first as one block, sections last.
pub fn project(ical: &ICalendar) -> String {
    let event = ical
        .components
        .iter()
        .find(|component| component.component_type == ICalendarComponentType::VEvent);

    let mut out = String::new();

    let _ = writeln!(out, "# iCalendar VEVENT as TOML, edited by tcal.");
    let _ = writeln!(out, "#");
    let _ = writeln!(
        out,
        "# Fill what you need; empty fields are ignored. Properties and"
    );
    let _ = writeln!(
        out,
        "# components tcal does not model (VALARM, VTIMEZONE, ...) are"
    );
    let _ = writeln!(out, "# kept verbatim, not shown here.");

    let collect = |field: &Field| -> Vec<&ICalendarEntry> {
        event
            .map(|event| {
                event
                    .entries
                    .iter()
                    .filter(|entry| entry.name.as_str() == field.name)
                    .collect()
            })
            .unwrap_or_default()
    };

    // The bare keys form one block with a shared comment column.
    let bare: Vec<&Field> = FIELDS
        .iter()
        .take_while(|field| field.kind.is_simple())
        .collect();
    let bare_lines: Vec<Line> = bare
        .iter()
        .flat_map(|field| field.lines(&collect(field)))
        .collect();
    let _ = writeln!(out);
    emit_lines(&mut out, &bare_lines, comment_column(bare_lines.iter()));

    // Each section is set off by a blank line and aligned within itself.
    for field in &FIELDS[bare.len()..] {
        let _ = writeln!(out);
        let lines = field.lines(&collect(field));
        emit_lines(&mut out, &lines, comment_column(lines.iter()));
    }

    out
}

/// Apply an edited TOML buffer onto the original iCalendar text.
///
/// The first `VEVENT`'s modeled fields are rewritten from the buffer
/// through a format-preserving editor (see [`crate::edit`]): only the
/// lines that actually changed are re-rendered, so unmodeled properties
/// (including the app-managed `UID` and `DTSTAMP`), sibling components,
/// folding, ordering and casing are all kept verbatim.
pub fn apply(original_src: &str, edited_toml: &str) -> Result<String> {
    let doc: DocumentMut = edited_toml.parse().map_err(TcalError::ParseToml)?;

    let mut cal = crate::edit::parse(original_src);
    let event = cal.component_mut("VEVENT").ok_or(TcalError::NoEvent)?;

    for field in FIELDS {
        event.set_all(field.name, &field.content_lines(&doc));
    }

    Ok(cal.to_string())
}

/// A projected line: a left side and an optional inline hint.
struct Line {
    lhs: String,
    hint: Option<String>,
}

/// Shape of a modeled property, driving both projection and emission.
enum Kind {
    /// Single value rendered as a bare key. `escape` is set for
    /// TEXT-typed properties (`UID`, `SUMMARY`), cleared for properties
    /// calcard does not unescape (`URL`, `STATUS`, `RRULE`, ...).
    Scalar { escape: bool },

    /// Free-form text, projected as a TOML multi-line literal
    /// (`DESCRIPTION`).
    Text,

    /// Repeated or multi-valued text, joined on `sep` in the iCalendar
    /// (`CATEGORIES`).
    List { sep: char },

    /// Date or date-time, projected as a friendly `YYYY-MM-DD[ HH:MM]`
    /// plus an adjacent `<key>_tz` time-zone key (`DTSTART`, `DTEND`).
    Date,

    /// Calendar address, projected without its `mailto:` scheme
    /// (`ORGANIZER`).
    CalAddress,

    /// Repeatable attendee with the common `CN` / `ROLE` / `PARTSTAT`
    /// parameters (`ATTENDEE`).
    Attendee,
}

impl Kind {
    /// A bare key (vs a `[[array]]` section).
    fn is_simple(&self) -> bool {
        !matches!(self, Kind::Attendee)
    }
}

/// A modeled VEVENT property and how it maps to TOML.
struct Field {
    /// TOML key.
    key: &'static str,

    /// Canonical iCalendar property name (matches calcard's `as_str`).
    name: &'static str,

    /// Inline hint shown next to the value, only where it is not
    /// self-evident (rendered as `  # <hint>`).
    hint: Option<&'static str>,

    /// Mapping shape.
    kind: Kind,
}

/// Shared hint for the friendly date keys.
const DATE_HINT: &str = "e.g. 2026-06-13 14:00, or 2026-06-13 for all-day; add UTC for UTC";

/// The modeled vocabulary. Everything outside this list is preserved
/// verbatim by [`apply`] but not surfaced in the scaffold.
///
/// The bare keys lead as one block (`description` last, so its literal
/// block sits at the end), and the sectioned `attendee` comes last: a
/// TOML document root ends at the first array-of-tables header.
const FIELDS: &[Field] = &[
    Field {
        key: "summary",
        name: "SUMMARY",
        hint: Some("required"),
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "dtstart",
        name: "DTSTART",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "dtend",
        name: "DTEND",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "duration",
        name: "DURATION",
        hint: Some("e.g. PT1H30M; an alternative to dtend"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "location",
        name: "LOCATION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "status",
        name: "STATUS",
        hint: Some("e.g. CONFIRMED, TENTATIVE, CANCELLED"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "class",
        name: "CLASS",
        hint: Some("e.g. PUBLIC, PRIVATE, CONFIDENTIAL"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "transparency",
        name: "TRANSP",
        hint: Some("e.g. OPAQUE, TRANSPARENT"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "priority",
        name: "PRIORITY",
        hint: Some("0 = undefined, 1 = highest, 9 = lowest"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "url",
        name: "URL",
        hint: Some("e.g. https://example.com/event"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "categories",
        name: "CATEGORIES",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "rrule",
        name: "RRULE",
        hint: Some("e.g. FREQ=WEEKLY;BYDAY=MO,WE,FR"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "organizer",
        name: "ORGANIZER",
        hint: Some("email or cal-address"),
        kind: Kind::CalAddress,
    },
    Field {
        key: "description",
        name: "DESCRIPTION",
        hint: None,
        kind: Kind::Text,
    },
    Field {
        key: "attendee",
        name: "ATTENDEE",
        hint: None,
        kind: Kind::Attendee,
    },
];

impl Field {
    /// Render this field into projected lines.
    fn lines(&self, entries: &[&ICalendarEntry]) -> Vec<Line> {
        match &self.kind {
            Kind::Scalar { .. } => {
                let value = entries
                    .first()
                    .map(|entry| scalar_text(entry))
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Text => {
                let value = entries
                    .first()
                    .map(|entry| scalar_text(entry))
                    .unwrap_or_default();
                text_lines(self.key, &value)
            }

            Kind::List { .. } => {
                let items: Vec<String> = entries
                    .iter()
                    .flat_map(|entry| {
                        entry
                            .values
                            .iter()
                            .filter_map(|value| value.as_text().map(str::to_owned))
                    })
                    .collect();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_array(&items)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Date => {
                let entry = entries.first();
                let value = entry
                    .and_then(|entry| entry.values.first())
                    .and_then(|value| value.as_partial_date_time())
                    .map(friendly_date)
                    .unwrap_or_default();
                let tz = entry
                    .and_then(|entry| entry.parameters(&ICalendarParameterName::Tzid).next())
                    .and_then(|value| value.as_text())
                    .unwrap_or_default();

                vec![
                    Line {
                        lhs: format!("{} = {}", self.key, toml_str(&value)),
                        hint: self.hint.map(str::to_owned),
                    },
                    Line {
                        lhs: format!("{}_tz = {}", self.key, toml_str(tz)),
                        hint: Some("e.g. America/New_York; empty for UTC or floating".to_owned()),
                    },
                ]
            }

            Kind::CalAddress => {
                let value = entries
                    .first()
                    .and_then(|entry| entry_text(entry))
                    .map(strip_mailto)
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Attendee => {
                let mut lines = Vec::new();

                if entries.is_empty() {
                    attendee_block(&mut lines, None);
                } else {
                    for entry in entries.iter().copied() {
                        attendee_block(&mut lines, Some(entry));
                    }
                }

                lines
            }
        }
    }

    /// This field's iCalendar content line(s) built from the edited
    /// `doc`, without an end of line, skipping empty values. Empty when
    /// the field is absent or blank, so [`crate::edit::Component::set_all`]
    /// removes it.
    fn content_lines(&self, doc: &DocumentMut) -> Vec<String> {
        let Some(item) = doc.get(self.key) else {
            return Vec::new();
        };

        let mut lines = Vec::new();

        match &self.kind {
            Kind::Scalar {
                escape: needs_escape,
            } => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    let value = if *needs_escape {
                        escape(value)
                    } else {
                        value.to_owned()
                    };
                    lines.push(format!("{}:{}", self.name, value));
                }
            }

            Kind::Text => {
                // Drop the trailing newline the literal block adds.
                if let Some(value) = item.as_str() {
                    let value = value.strip_suffix('\n').unwrap_or(value);
                    if !value.is_empty() {
                        lines.push(format!("{}:{}", self.name, escape(value)));
                    }
                }
            }

            Kind::List { sep } => {
                if let Some(array) = item.as_array() {
                    let parts: Vec<String> = array
                        .iter()
                        .filter_map(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .map(escape)
                        .collect();

                    if !parts.is_empty() {
                        lines.push(format!("{}:{}", self.name, parts.join(&sep.to_string())));
                    }
                }
            }

            Kind::Date => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    let tz = doc
                        .get(&format!("{}_tz", self.key))
                        .and_then(|item| item.as_str())
                        .filter(|value| !value.is_empty());
                    lines.push(date_line(self.name, value, tz));
                }
            }

            Kind::CalAddress => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push(format!("{}:{}", self.name, ensure_mailto(value)));
                }
            }

            Kind::Attendee => {
                for table in tables(item) {
                    let Some(value) = table
                        .get("value")
                        .and_then(|item| item.as_str())
                        .filter(|value| !value.is_empty())
                    else {
                        continue;
                    };

                    let mut line = self.name.to_string();
                    push_param(&mut line, "CN", table.get("cn"));
                    push_param(&mut line, "ROLE", table.get("role"));
                    push_param(&mut line, "PARTSTAT", table.get("partstat"));
                    line.push(':');
                    line.push_str(&ensure_mailto(value));
                    lines.push(line);
                }
            }
        }

        lines
    }
}

/// The column at which a block's inline `#` comments are aligned: the
/// widest left side among the lines that carry a hint.
fn comment_column<'a>(lines: impl Iterator<Item = &'a Line>) -> usize {
    lines
        .filter(|line| line.hint.is_some())
        .map(|line| line.lhs.len())
        .max()
        .unwrap_or(0)
}

/// Emit lines, padding hinted ones so their `#` lands on `column`.
fn emit_lines(out: &mut String, lines: &[Line], column: usize) {
    for line in lines {
        match &line.hint {
            Some(hint) => {
                let _ = writeln!(out, "{:<width$}  # {hint}", line.lhs, width = column);
            }
            None => {
                let _ = writeln!(out, "{}", line.lhs);
            }
        }
    }
}

/// Project a `description` as a TOML multi-line literal: `''''''` when
/// empty, a `'''` block otherwise. Literal strings cannot contain
/// `'''`, so such a value falls back to a basic string.
fn text_lines(key: &str, value: &str) -> Vec<Line> {
    if value.is_empty() {
        return vec![Line {
            lhs: format!("{key} = ''''''"),
            hint: None,
        }];
    }

    if value.contains("'''") {
        return vec![Line {
            lhs: format!("{key} = {}", toml_str(value)),
            hint: None,
        }];
    }

    let mut lines = vec![Line {
        lhs: format!("{key} = '''"),
        hint: None,
    }];
    for content in value.lines() {
        lines.push(Line {
            lhs: content.to_owned(),
            hint: None,
        });
    }
    lines.push(Line {
        lhs: "'''".into(),
        hint: None,
    });
    lines
}

/// Render one `[[attendee]]` block, filled or empty.
fn attendee_block(lines: &mut Vec<Line>, entry: Option<&ICalendarEntry>) {
    lines.push(Line {
        lhs: "[[attendee]]".into(),
        hint: None,
    });

    let value = entry
        .and_then(entry_text)
        .map(strip_mailto)
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("value = {}", toml_str(value)),
        hint: Some("email or cal-address".to_owned()),
    });

    let cn = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Cn))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("cn = {}", toml_str(&cn)),
        hint: None,
    });

    let role = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Role))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("role = {}", toml_str(&role)),
        hint: Some("e.g. CHAIR, REQ-PARTICIPANT, OPT-PARTICIPANT".to_owned()),
    });

    let partstat = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Partstat))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("partstat = {}", toml_str(&partstat)),
        hint: Some("e.g. NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE".to_owned()),
    });
}

/// Build an iCalendar date line from a friendly value and optional time
/// zone, falling back to the value verbatim when it is not in the
/// friendly form (so power users can type a raw iCalendar value).
fn date_line(name: &str, value: &str, tz: Option<&str>) -> String {
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

/// Parse a friendly `YYYY-MM-DD[ HH:MM[:SS]][ UTC]` into its iCalendar
/// digit parts: the date (`YYYYMMDD`), an optional time (`HHMMSS`, or
/// `None` for an all-day date), and whether it is UTC.
fn parse_friendly_date(value: &str) -> Option<(String, Option<String>, bool)> {
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
/// appending ` UTC` for a UTC value. A value with no time of day is an
/// all-day date and renders as `YYYY-MM-DD`.
fn friendly_date(dt: &PartialDateTime) -> String {
    let (Some(year), Some(month), Some(day)) = (dt.year, dt.month, dt.day) else {
        return String::new();
    };
    let date = format!("{year:04}-{month:02}-{day:02}");

    let (Some(hour), Some(minute)) = (dt.hour, dt.minute) else {
        return date;
    };
    let mut out = format!("{date} {hour:02}:{minute:02}");

    if let Some(second) = dt.second.filter(|second| *second != 0) {
        let _ = write!(out, ":{second:02}");
    }
    if matches!((dt.tz_hour, dt.tz_minute), (Some(0), Some(0))) {
        out.push_str(" UTC");
    }

    out
}

/// Append `;NAME=value` to `line` when the table entry is non-empty,
/// quoting the value when it carries a parameter delimiter.
fn push_param(line: &mut String, name: &str, item: Option<&Item>) {
    let Some(value) = item
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
    else {
        return;
    };

    line.push(';');
    line.push_str(name);
    line.push('=');

    if value.contains([',', ';', ':', '"']) {
        line.push('"');
        line.push_str(&value.replace('"', ""));
        line.push('"');
    } else {
        line.push_str(value);
    }
}

/// Collect the TOML tables addressed by an array-of-tables (`[[key]]`)
/// or an inline array of inline tables.
fn tables(item: &Item) -> Vec<&dyn TableLike> {
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

/// First value of an entry as text, falling back to its owned text form
/// for typed values (integers, durations, recurrence rules, ...).
fn scalar_text(entry: &ICalendarEntry) -> String {
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
fn entry_text(entry: &ICalendarEntry) -> Option<&str> {
    entry.values.first().and_then(|value| value.as_text())
}

/// First value of a named parameter as owned text.
fn param(entry: &ICalendarEntry, name: &ICalendarParameterName) -> Option<String> {
    entry
        .parameters(name)
        .next()
        .and_then(|value| value.as_text())
        .map(str::to_owned)
}

/// A calendar address without its `mailto:` scheme, for display.
fn strip_mailto(value: &str) -> &str {
    value
        .strip_prefix("mailto:")
        .or_else(|| value.strip_prefix("MAILTO:"))
        .unwrap_or(value)
}

/// A calendar address with a scheme: a bare address gains `mailto:`.
fn ensure_mailto(value: &str) -> String {
    if value.contains(':') {
        value.to_owned()
    } else {
        format!("mailto:{value}")
    }
}

/// Render a string as a quoted, escaped TOML scalar.
fn toml_str(value: &str) -> String {
    toml_edit::Value::from(value).to_string().trim().to_string()
}

/// Render strings as a TOML array.
fn toml_array<S: AsRef<str>>(items: &[S]) -> String {
    let mut array = toml_edit::Array::new();

    for item in items {
        array.push(item.as_ref());
    }

    array.to_string().trim().to_string()
}

/// Escape an iCalendar text value per RFC 5545 section 3.3.11.
fn escape(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use crate::ical;

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
        VERSION:2.0\r\n\
        PRODID:-//Test//EN\r\n\
        BEGIN:VEVENT\r\n\
        UID:abc@example\r\n\
        DTSTAMP:20260101T000000Z\r\n\
        DTSTART;TZID=America/New_York:20260613T140000\r\n\
        DTEND;TZID=America/New_York:20260613T150000\r\n\
        SUMMARY:Team sync\r\n\
        LOCATION:Room 1\r\n\
        STATUS:CONFIRMED\r\n\
        CATEGORIES:work,meeting\r\n\
        ATTENDEE;CN=Jane Doe;ROLE=REQ-PARTICIPANT;PARTSTAT=ACCEPTED:mailto:jane@example.com\r\n\
        X-CUSTOM:keep me verbatim\r\n\
        BEGIN:VALARM\r\n\
        ACTION:DISPLAY\r\n\
        TRIGGER:-PT15M\r\n\
        END:VALARM\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    #[test]
    fn project_prefills_known_fields() {
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project(&ical);

        assert!(toml.contains("summary = \"Team sync\""));
        assert!(toml.contains("dtstart = \"2026-06-13 14:00\""));
        assert!(toml.contains("dtstart_tz = \"America/New_York\""));
        assert!(toml.contains("location = \"Room 1\""));
        assert!(toml.contains("value = \"jane@example.com\""));
        assert!(toml.contains("cn = \"Jane Doe\""));
        // Unmodeled properties and components never appear (the header
        // comment names them, so probe their data lines, not the words).
        assert!(!toml.contains("keep me verbatim"));
        assert!(!toml.contains("TRIGGER"));
        assert!(!toml.contains("DTSTAMP"));
    }

    #[test]
    fn blank_project_layout() {
        let toml = super::project(&Default::default());

        // summary leads; dtstart before dtend; description is the last
        // bare key; attendee is a section. uid is app-managed, not shown.
        assert!(!toml.contains("uid"));
        assert!(toml.find("summary =").unwrap() < toml.find("dtstart =").unwrap());
        assert!(toml.find("dtstart =").unwrap() < toml.find("dtend =").unwrap());
        assert!(toml.find("description =").unwrap() < toml.find("[[attendee]]").unwrap());

        // Empty, uncommented fields; description as an empty literal.
        assert!(toml.contains("summary = \"\""));
        assert!(toml.contains("description = ''''''"));
        assert!(!toml.contains("#summary"));

        // SUMMARY is flagged required; hints use the e.g. form.
        assert!(toml.contains("# required"));
        assert!(toml.contains("# e.g. FREQ=WEEKLY"));
        assert!(toml.contains("for all-day"));
    }

    #[test]
    fn blank_bare_hints_share_a_column() {
        let toml = super::project(&Default::default());

        let column = |needle: &str| -> usize {
            let line = toml.lines().find(|line| line.contains(needle)).unwrap();
            line.find('#').unwrap()
        };

        // summary, duration, status, priority all align in the bare block.
        assert_eq!(column("duration ="), column("summary ="));
        assert_eq!(column("duration ="), column("status ="));
        assert_eq!(column("duration ="), column("priority ="));
    }

    #[test]
    fn apply_roundtrip_preserves_unknown_and_components() {
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project(&ical);

        let out = super::apply(SAMPLE, &toml).unwrap();

        assert!(out.contains("SUMMARY:Team sync"));
        assert!(out.contains("DTSTART;TZID=America/New_York:20260613T140000"));
        assert!(out.contains("mailto:jane@example.com"));
        // The unmodeled property and the VALARM survive verbatim.
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
        assert!(out.contains("BEGIN:VALARM"));
        assert!(out.contains("TRIGGER:-PT15M"));
        // Bookkeeping is preserved although it is not modeled.
        assert!(out.contains("DTSTAMP:20260101T000000Z"));
    }

    #[test]
    fn apply_projection_is_a_no_op() {
        // Projecting then applying an untouched buffer must reproduce the
        // source byte-for-byte: the minimal-diff guarantee at its limit.
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project(&ical);

        assert_eq!(super::apply(SAMPLE, &toml).unwrap(), SAMPLE);
    }

    #[test]
    fn apply_changes_only_the_edited_line() {
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project(&ical).replace("Team sync", "Team lunch");

        let out = super::apply(SAMPLE, &toml).unwrap();

        assert_eq!(
            out,
            SAMPLE.replace("SUMMARY:Team sync", "SUMMARY:Team lunch")
        );
    }

    #[test]
    fn apply_ignores_empty_fields() {
        // A whole blank form drops every modeled field yet keeps the
        // unknown property and the sibling component.
        let blank = super::project(&Default::default());

        let out = super::apply(SAMPLE, &blank).unwrap();

        assert!(!out.contains("SUMMARY:"));
        assert!(!out.contains("DTSTART"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
        assert!(out.contains("BEGIN:VALARM"));
    }

    #[test]
    fn uid_is_hidden_and_app_managed() {
        let ical = ical::parse(SAMPLE).unwrap();

        // Hidden from the form.
        let toml = super::project(&ical);
        assert!(!toml.contains("uid"));

        // Preserved on round-trip, and not overridable from the buffer.
        let out = super::apply(SAMPLE, "uid = \"hacked\"\n").unwrap();
        assert!(out.contains("UID:abc@example"));
        assert!(!out.contains("hacked"));
    }

    #[test]
    fn apply_edits_modeled_field() {
        let edited = "summary = \"New title\"\n";

        let out = super::apply(SAMPLE, edited).unwrap();

        assert!(out.contains("SUMMARY:New title"));
        assert!(!out.contains("Team sync"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_renders_all_day_and_utc_dates() {
        let all_day = super::apply(SAMPLE, "dtstart = \"2026-12-25\"\n").unwrap();
        assert!(all_day.contains("DTSTART;VALUE=DATE:20261225"));

        let utc = super::apply(SAMPLE, "dtstart = \"2026-06-13 14:00 UTC\"\n").unwrap();
        assert!(utc.contains("DTSTART:20260613T140000Z"));

        let zoned = super::apply(
            SAMPLE,
            "dtstart = \"2026-06-13 09:30\"\ndtstart_tz = \"Europe/Paris\"\n",
        )
        .unwrap();
        assert!(zoned.contains("DTSTART;TZID=Europe/Paris:20260613T093000"));
    }
}
