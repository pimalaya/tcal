//! The modeled component vocabulary: each [`Spec`]'s [`Field`]s, and how
//! every [`Kind`] of value projects to and parses from TOML.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use calcard::icalendar::{ICalendarEntry, ICalendarParameterName};
use toml_edit::TableLike;

use crate::template::{
    datetime::{DATE_HINT, date_line, ical_date, is_utc, offset_text, toml_date, toml_date_line},
    duration::{duration_lines, duration_value},
    line::Line,
    recurrence::{recur_lines, recur_rule},
    util::{
        ensure_mailto, entry_text, escape, push_param, scalar_text, strip_mailto, tables,
        toml_array, toml_number, toml_str, value_text,
    },
};

/// Shape of a modeled property, driving both projection and emission.
pub(crate) enum Kind {
    /// Bare key; `escape` set for TEXT properties calcard unescapes.
    Scalar { escape: bool },

    /// Closed RFC 5545 vocabulary (`STATUS`, `CLASS`, ...): listed lowercase
    /// in the hint, uppercased to canonical form on export.
    Enum,

    /// Integer, rendered as a bare TOML number.
    Number,

    /// Multi-valued text joined on `sep` (`CATEGORIES`).
    List { sep: char },

    /// Date or date-time as a friendly value plus an adjacent `<key>-tz` key.
    Date,

    /// Calendar address, projected without its `mailto:` scheme.
    CalAddress,

    /// UTC offset (`TZOFFSETFROM`/`TZOFFSETTO`), projected as `±HHMM`.
    Offset,

    /// Repeatable attendee with `CN` / `ROLE` / `PARTSTAT` parameters.
    Attendee,

    /// Recurrence rule as dotted `<key>.*` keys (see [`recur_lines`]).
    Recur,

    /// Duration as dotted `<key>.{week,day,...}` keys, sign implied by
    /// context (`negative` for an alarm trigger); see [`duration_lines`].
    Duration { negative: bool },
}

impl Kind {
    /// A bare/inline key, vs the sectioned attendee array-of-tables.
    pub(crate) fn is_simple(&self) -> bool {
        !matches!(self, Kind::Attendee)
    }
}

/// A modeled property and how it maps to TOML: TOML `key`, iCalendar `name`,
/// optional `hint`, and mapping `kind`.
pub(crate) struct Field {
    pub(crate) key: &'static str,
    pub(crate) name: &'static str,
    hint: Option<&'static str>,
    pub(crate) kind: Kind,
}

/// The modeled `VEVENT` fields, grouped by shape: the bare scalar keys
/// (the headline `summary`/`description` leading), then the dates, the
/// duration, and the recurrence, each its own group, with the sectioned
/// `attendee` last (a TOML array-of-tables header must follow all of its
/// table's bare keys).
const FIELDS: &[Field] = &[
    Field {
        key: "summary",
        name: "SUMMARY",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "description",
        name: "DESCRIPTION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "categories",
        name: "CATEGORIES",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "location",
        name: "LOCATION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "url",
        name: "URL",
        hint: Some("https://example.com/event"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "organizer",
        name: "ORGANIZER",
        hint: Some("email address"),
        kind: Kind::CalAddress,
    },
    Field {
        key: "class",
        name: "CLASS",
        hint: Some("public, private, confidential"),
        kind: Kind::Enum,
    },
    Field {
        key: "priority",
        name: "PRIORITY",
        hint: Some("0 = undefined, 1 = highest, 9 = lowest"),
        kind: Kind::Number,
    },
    Field {
        key: "status",
        name: "STATUS",
        hint: Some("confirmed, tentative, cancelled"),
        kind: Kind::Enum,
    },
    Field {
        key: "transparency",
        name: "TRANSP",
        hint: Some("opaque, transparent"),
        kind: Kind::Enum,
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "date-end",
        name: "DTEND",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "duration",
        name: "DURATION",
        hint: Some("event length"),
        kind: Kind::Duration { negative: false },
    },
    Field {
        key: "recurrence",
        name: "RRULE",
        hint: None,
        kind: Kind::Recur,
    },
    Field {
        key: "attendee",
        name: "ATTENDEE",
        hint: None,
        kind: Kind::Attendee,
    },
];

/// The modeled `VALARM` vocabulary, projected as repeatable `[[alarm]]`
/// blocks. Kept to plain scalars: alarm values are short codes and
/// durations, not dates or addresses.
const VALARM_FIELDS: &[Field] = &[
    Field {
        key: "summary",
        name: "SUMMARY",
        hint: Some("subject line for an email alarm"),
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "description",
        name: "DESCRIPTION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "action",
        name: "ACTION",
        hint: Some("display, email, audio"),
        kind: Kind::Enum,
    },
    Field {
        key: "repeat",
        name: "REPEAT",
        hint: Some("how many extra times to fire: 2"),
        kind: Kind::Number,
    },
    Field {
        key: "trigger",
        name: "TRIGGER",
        hint: Some("before the event"),
        kind: Kind::Duration { negative: true },
    },
    Field {
        key: "duration",
        name: "DURATION",
        hint: Some("with repeat"),
        kind: Kind::Duration { negative: false },
    },
];

/// Modeled `VTODO` fields: like an event, but with `due`/`completed`/
/// `percent` instead of `dtend`/`transparency`.
const TODO_FIELDS: &[Field] = &[
    Field {
        key: "summary",
        name: "SUMMARY",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "description",
        name: "DESCRIPTION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "categories",
        name: "CATEGORIES",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "location",
        name: "LOCATION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "url",
        name: "URL",
        hint: Some("https://example.com/task"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "organizer",
        name: "ORGANIZER",
        hint: Some("email address"),
        kind: Kind::CalAddress,
    },
    Field {
        key: "class",
        name: "CLASS",
        hint: Some("public, private, confidential"),
        kind: Kind::Enum,
    },
    Field {
        key: "priority",
        name: "PRIORITY",
        hint: Some("0 = undefined, 1 = highest, 9 = lowest"),
        kind: Kind::Number,
    },
    Field {
        key: "status",
        name: "STATUS",
        hint: Some("needs-action, in-process, completed, cancelled"),
        kind: Kind::Enum,
    },
    Field {
        key: "percent",
        name: "PERCENT-COMPLETE",
        hint: Some("0 to 100"),
        kind: Kind::Number,
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "date-due",
        name: "DUE",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "date-completed",
        name: "COMPLETED",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "duration",
        name: "DURATION",
        hint: Some("alternative to a due date"),
        kind: Kind::Duration { negative: false },
    },
    Field {
        key: "recurrence",
        name: "RRULE",
        hint: None,
        kind: Kind::Recur,
    },
    Field {
        key: "attendee",
        name: "ATTENDEE",
        hint: None,
        kind: Kind::Attendee,
    },
];

/// Modeled `VJOURNAL` fields: a dated note, no times or alarms.
const JOURNAL_FIELDS: &[Field] = &[
    Field {
        key: "summary",
        name: "SUMMARY",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "description",
        name: "DESCRIPTION",
        hint: None,
        kind: Kind::Scalar { escape: true },
    },
    Field {
        key: "categories",
        name: "CATEGORIES",
        hint: None,
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "url",
        name: "URL",
        hint: None,
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "organizer",
        name: "ORGANIZER",
        hint: Some("email address"),
        kind: Kind::CalAddress,
    },
    Field {
        key: "class",
        name: "CLASS",
        hint: Some("public, private, confidential"),
        kind: Kind::Enum,
    },
    Field {
        key: "status",
        name: "STATUS",
        hint: Some("draft, final, cancelled"),
        kind: Kind::Enum,
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "recurrence",
        name: "RRULE",
        hint: None,
        kind: Kind::Recur,
    },
    Field {
        key: "attendee",
        name: "ATTENDEE",
        hint: None,
        kind: Kind::Attendee,
    },
];

/// Modeled `VFREEBUSY` fields: a busy-time report over a window.
const FREEBUSY_FIELDS: &[Field] = &[
    Field {
        key: "organizer",
        name: "ORGANIZER",
        hint: Some("email address"),
        kind: Kind::CalAddress,
    },
    Field {
        key: "periods",
        name: "FREEBUSY",
        hint: Some("20260613T140000Z/PT1H"),
        kind: Kind::List { sep: ',' },
    },
    Field {
        key: "url",
        name: "URL",
        hint: None,
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "date-end",
        name: "DTEND",
        hint: Some(DATE_HINT),
        kind: Kind::Date,
    },
    Field {
        key: "attendee",
        name: "ATTENDEE",
        hint: None,
        kind: Kind::Attendee,
    },
];

/// Modeled fields of a `STANDARD` / `DAYLIGHT` time-zone rule.
const TZRULE_FIELDS: &[Field] = &[
    Field {
        key: "name",
        name: "TZNAME",
        hint: Some("CET"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "offset-from",
        name: "TZOFFSETFROM",
        hint: Some("+0200"),
        kind: Kind::Offset,
    },
    Field {
        key: "offset-to",
        name: "TZOFFSETTO",
        hint: Some("+0100"),
        kind: Kind::Offset,
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some("local start: 1996-10-27T03:00:00"),
        kind: Kind::Date,
    },
    Field {
        key: "recurrence",
        name: "RRULE",
        hint: None,
        kind: Kind::Recur,
    },
];

/// Modeled `VTIMEZONE` fields; its transitions are the nested `standard`
/// and `daylight` sub-components.
const TIMEZONE_FIELDS: &[Field] = &[Field {
    key: "id",
    name: "TZID",
    hint: Some("Europe/Paris"),
    kind: Kind::Scalar { escape: false },
}];

/// A modeled iCalendar component: its TOML key, its iCalendar name, its
/// fields, and its nested child components.
pub(crate) struct Spec {
    /// TOML array-of-tables key (e.g. `event`).
    pub(crate) key: &'static str,
    /// iCalendar component name (e.g. `VEVENT`).
    pub(crate) name: &'static str,
    /// Modeled fields, in projection order.
    pub(crate) fields: &'static [Field],
    /// Nested child component specs (e.g. a `VEVENT`'s `VALARM`s).
    pub(crate) children: &'static [&'static Spec],
}

static VALARM: Spec = Spec {
    key: "alarm",
    name: "VALARM",
    fields: VALARM_FIELDS,
    children: &[],
};

static STANDARD: Spec = Spec {
    key: "standard",
    name: "STANDARD",
    fields: TZRULE_FIELDS,
    children: &[],
};

static DAYLIGHT: Spec = Spec {
    key: "daylight",
    name: "DAYLIGHT",
    fields: TZRULE_FIELDS,
    children: &[],
};

pub(crate) static VEVENT: Spec = Spec {
    key: "event",
    name: "VEVENT",
    fields: FIELDS,
    children: &[&VALARM],
};

static VTODO: Spec = Spec {
    key: "todo",
    name: "VTODO",
    fields: TODO_FIELDS,
    children: &[&VALARM],
};

static VJOURNAL: Spec = Spec {
    key: "journal",
    name: "VJOURNAL",
    fields: JOURNAL_FIELDS,
    children: &[],
};

static VFREEBUSY: Spec = Spec {
    key: "free-busy",
    name: "VFREEBUSY",
    fields: FREEBUSY_FIELDS,
    children: &[],
};

static VTIMEZONE: Spec = Spec {
    key: "timezone",
    name: "VTIMEZONE",
    fields: TIMEZONE_FIELDS,
    children: &[&STANDARD, &DAYLIGHT],
};

/// The top-level component types tcal models, in projection order.
/// Everything else is preserved verbatim.
pub(crate) static TOP_LEVEL: &[&Spec] = &[&VEVENT, &VTODO, &VJOURNAL, &VFREEBUSY, &VTIMEZONE];

impl Field {
    /// Render this field into projected lines.
    pub(crate) fn lines(&self, entries: &[&ICalendarEntry]) -> Vec<Line> {
        match &self.kind {
            Kind::Scalar { .. } | Kind::Enum => {
                let value = entries
                    .first()
                    .map(|entry| scalar_text(entry))
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Number => {
                let value = entries
                    .first()
                    .map(|entry| scalar_text(entry))
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_number(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::List { .. } => {
                let items: Vec<String> = entries
                    .iter()
                    .flat_map(|entry| entry.values.iter().filter_map(value_text))
                    .collect();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_array(&items)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Date => {
                let entry = entries.first();
                let dt = entry
                    .and_then(|entry| entry.values.first())
                    .and_then(|value| value.as_partial_date_time());
                let tzid = entry
                    .and_then(|entry| entry.parameters(&ICalendarParameterName::Tzid).next())
                    .and_then(|value| value.as_text())
                    .filter(|zone| !zone.is_empty());

                // A complete value projects as a native TOML date or
                // date-time; a partial one (yearless or year-only) falls back
                // to a quoted basic ISO 8601 string. The named zone, if any,
                // is kept beside it.
                let (rhs, zone) = match dt {
                    Some(dt) => match toml_date(dt) {
                        Some(native) => {
                            let zone = (!is_utc(dt)).then_some(tzid).flatten();
                            (native.to_string(), zone)
                        }
                        None => (toml_str(&ical_date(dt)), None),
                    },
                    None => (toml_str(""), None),
                };

                let mut lines = vec![Line {
                    lhs: format!("{} = {}", self.key, rhs),
                    hint: self.hint.map(str::to_owned),
                }];

                // The zone key is kept only for a named zone (a UTC or
                // floating value needs none), and shown empty in the blank
                // scaffold as the affordance to add one.
                if zone.is_some() || dt.is_none() {
                    lines.push(Line {
                        lhs: format!("{}-tz = {}", self.key, toml_str(zone.unwrap_or_default())),
                        hint: Some("America/New_York; empty for UTC or floating".to_owned()),
                    });
                }

                lines
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

            Kind::Offset => {
                let value = entries
                    .first()
                    .and_then(|entry| entry.values.first())
                    .and_then(|value| value.as_partial_date_time())
                    .map(offset_text)
                    .unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Recur => recur_lines(entries.first().copied(), self.key),

            Kind::Duration { .. } => duration_lines(entries.first().copied(), self.key, self.hint),

            // Attendees need their parent's TOML prefix, so they are
            // rendered by `attendee_section`, not here.
            Kind::Attendee => unreachable!("attendee is rendered with its parent prefix"),
        }
    }

    /// This field's iCalendar content line(s) built from a TOML table
    /// (the edited document, or a single `[[alarm]]` table), without an
    /// end of line, skipping empty values. Empty when the field is absent
    /// or blank, so [`crate::edit::tree::Component::set_all`] removes it.
    pub(crate) fn content_lines(&self, source: &dyn TableLike) -> Vec<String> {
        let Some(item) = source.get(self.key) else {
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

            Kind::Enum => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push(format!("{}:{}", self.name, value.to_uppercase()));
                }
            }

            Kind::Number => {
                let value = item
                    .as_integer()
                    .map(|number| number.to_string())
                    .or_else(|| {
                        item.as_str()
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned)
                    });
                if let Some(value) = value {
                    lines.push(format!("{}:{}", self.name, value));
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
                let tz = source
                    .get(&format!("{}-tz", self.key))
                    .and_then(|item| item.as_str())
                    .filter(|value| !value.is_empty());

                if let Some(dtm) = item.as_datetime() {
                    lines.push(toml_date_line(self.name, dtm, tz));
                } else if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push(date_line(self.name, value, tz));
                }
            }

            Kind::CalAddress => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push(format!("{}:{}", self.name, ensure_mailto(value)));
                }
            }

            Kind::Offset => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push(format!("{}:{}", self.name, value));
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
                    push_param(&mut line, "CN", table.get("display-name"), false);
                    push_param(&mut line, "ROLE", table.get("role"), true);
                    push_param(&mut line, "PARTSTAT", table.get("status"), true);
                    line.push(':');
                    line.push_str(&ensure_mailto(value));
                    lines.push(line);
                }
            }

            Kind::Recur => {
                if let Some(table) = item.as_table_like()
                    && let Some(rule) = recur_rule(table)
                {
                    lines.push(format!("{}:{}", self.name, rule));
                }
            }

            Kind::Duration { negative } => {
                if let Some(table) = item.as_table_like()
                    && let Some(value) = duration_value(table, *negative)
                {
                    lines.push(format!("{}:{}", self.name, value));
                }
            }
        }

        lines
    }
}
