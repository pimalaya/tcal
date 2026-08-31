//! # Modelled vocabulary
//!
//! Each [`Spec`]'s [`Field`]s, and how every [`Kind`] of value projects to and
//! parses from TOML.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use ical::tree::line::IcalLine;
use toml_edit::TableLike;

use crate::{
    ical::TcalProp,
    template::{
        datetime::{DATE_HINT, date_line, is_utc, toml_date, toml_date_line},
        duration::{duration_lines, duration_value},
        line::Line,
        patch::{Content, ensure_mailto, escape, strip_mailto},
        recurrence::{recur_lines, recur_rule},
        toml::{push_param, tables, toml_array, toml_number, toml_str},
    },
};

/// Shape of a modeled property, driving both projection and emission.
pub(crate) enum Kind {
    /// Bare key; `escape` marks the TEXT properties escaped on the wire.
    Scalar { escape: bool },
    /// Closed RFC 5545 vocabulary (`STATUS`, `CLASS`, ...).
    ///
    /// Listed lowercase in the hint, uppercased to canonical form on export.
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
    ///
    /// `statuses` are the participation statuses the component defines, RFC
    /// 5545 section 3.2.12 closing `PARTSTAT` differently for an event, a
    /// to-do and a journal.
    Attendee { statuses: &'static str },
    /// Recurrence rule as dotted `<key>.*` keys (see [`recur_lines`]).
    Recur,
    /// Duration as dotted `<key>.{week,day,...}` keys, see [`duration_lines`].
    ///
    /// The sign is implied by context, `negative` for an alarm trigger.
    Duration { negative: bool },
}

impl Kind {
    /// A bare/inline key, vs the sectioned attendee array-of-tables.
    pub(crate) fn is_simple(&self) -> bool {
        !matches!(self, Kind::Attendee { .. })
    }
}

/// A modeled property and how it maps to TOML.
pub(crate) struct Field {
    /// TOML key.
    pub(crate) key: &'static str,
    /// Canonical iCalendar property name.
    pub(crate) name: &'static str,
    /// Inline hint shown next to the value, rendered as ` # <hint>`.
    hint: Option<&'static str>,
    /// Mapping shape.
    pub(crate) kind: Kind,
}

/// The modeled `VEVENT` fields, grouped by shape.
///
/// The bare scalars lead, `summary` and `description` first, then the
/// dates, the duration and the recurrence, with the sectioned `attendee`
/// last: a TOML array-of-tables header must follow its table's bare keys.
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
        kind: Kind::Attendee {
            statuses: "needs-action, accepted, declined, tentative, delegated",
        },
    },
];

/// The modeled `VALARM` vocabulary, as repeatable `[[alarm]]` blocks.
///
/// Kept to plain scalars: alarm values are short codes and durations, not
/// dates or addresses.
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

/// Modeled `VTODO` fields.
///
/// Like an event, but with `due`/`completed`/`percent` instead of
/// `dtend`/`transparency`.
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
        kind: Kind::Attendee {
            statuses: "needs-action, accepted, declined, tentative, delegated, completed, in-process",
        },
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
        kind: Kind::Attendee {
            statuses: "needs-action, accepted, declined",
        },
    },
];

/// Modeled `VFREEBUSY` fields: a busy-time report over a window.
///
/// RFC 5545 section 3.2.12 closes `PARTSTAT` per component and defines no
/// free/busy set, so its attendee is offered an event's.
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
        kind: Kind::Attendee {
            statuses: "needs-action, accepted, declined, tentative, delegated",
        },
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

/// Modeled `VTIMEZONE` fields.
///
/// Its transitions are the nested `standard` and `daylight` sub-components.
const TIMEZONE_FIELDS: &[Field] = &[Field {
    key: "id",
    name: "TZID",
    hint: Some("Europe/Paris"),
    kind: Kind::Scalar { escape: false },
}];

/// A modeled iCalendar component and how it maps to TOML.
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

/// The `VALARM` spec, nested in an event or a to-do.
static VALARM: Spec = Spec {
    key: "alarm",
    name: "VALARM",
    fields: VALARM_FIELDS,
    children: &[],
};

/// The `STANDARD` spec, a time zone's rule outside daylight saving.
static STANDARD: Spec = Spec {
    key: "standard",
    name: "STANDARD",
    fields: TZRULE_FIELDS,
    children: &[],
};

/// The `DAYLIGHT` spec, a time zone's daylight saving rule.
static DAYLIGHT: Spec = Spec {
    key: "daylight",
    name: "DAYLIGHT",
    fields: TZRULE_FIELDS,
    children: &[],
};

/// The `VEVENT` spec, the one a flat, unsectioned document defaults to.
pub(crate) static VEVENT: Spec = Spec {
    key: "event",
    name: "VEVENT",
    fields: FIELDS,
    children: &[&VALARM],
};

/// The `VTODO` spec, a task with a due date and a completion state.
static VTODO: Spec = Spec {
    key: "todo",
    name: "VTODO",
    fields: TODO_FIELDS,
    children: &[&VALARM],
};

/// The `VJOURNAL` spec, a dated note.
static VJOURNAL: Spec = Spec {
    key: "journal",
    name: "VJOURNAL",
    fields: JOURNAL_FIELDS,
    children: &[],
};

/// The `VFREEBUSY` spec, a busy-time report over a window.
static VFREEBUSY: Spec = Spec {
    key: "free-busy",
    name: "VFREEBUSY",
    fields: FREEBUSY_FIELDS,
    children: &[],
};

/// The `VTIMEZONE` spec, a time zone and its transition rules.
static VTIMEZONE: Spec = Spec {
    key: "timezone",
    name: "VTIMEZONE",
    fields: TIMEZONE_FIELDS,
    children: &[&STANDARD, &DAYLIGHT],
};

/// The top-level component types tCal models, in projection order.
///
/// Everything else is preserved verbatim.
pub(crate) static TOP_LEVEL: &[&Spec] = &[&VEVENT, &VTODO, &VJOURNAL, &VFREEBUSY, &VTIMEZONE];

impl Field {
    /// Render this field into projected lines.
    pub(crate) fn lines(&self, entries: &[&IcalLine<'_>]) -> Vec<Line> {
        match &self.kind {
            Kind::Scalar { .. } | Kind::Enum => {
                let value = entries.first().map(|line| line.text()).unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Number => {
                let value = entries.first().map(|line| line.text()).unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_number(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::List { .. } => {
                let values: Vec<String> = entries.iter().flat_map(|line| line.items()).collect();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_array(&values)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Date => {
                let entry = entries.first();
                let value = entry.map(|line| line.text()).unwrap_or_default();
                let dtm = toml_date(&value);
                let tzid = entry
                    .and_then(|line| line.param_value("TZID"))
                    .filter(|zone| !zone.is_empty());

                // NOTE: a value not in digit form falls back to the string
                // the calendar wrote, so a form the model cannot read still
                // round-trips.
                let (rhs, zone) = match &dtm {
                    Some(dtm) => ((*dtm).to_string(), (!is_utc(dtm)).then_some(tzid).flatten()),
                    None => (toml_str(&value), tzid.filter(|_| !value.is_empty())),
                };

                let mut lines = vec![Line {
                    lhs: format!("{} = {}", self.key, rhs),
                    hint: self.hint.map(str::to_owned),
                }];

                // NOTE: a UTC or floating value needs no zone key, but the
                // blank scaffold shows an empty one as the affordance to
                // add a zone.
                if zone.is_some() || entry.is_none() {
                    lines.push(Line {
                        lhs: format!("{}-tz = {}", self.key, toml_str(&zone.unwrap_or_default())),
                        hint: Some("America/New_York; empty for UTC or floating".to_owned()),
                    });
                }

                lines
            }

            Kind::CalAddress => {
                let value = entries.first().map(|line| line.text()).unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(strip_mailto(&value))),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Offset => {
                let value = entries.first().map(|line| line.text()).unwrap_or_default();
                vec![Line {
                    lhs: format!("{} = {}", self.key, toml_str(&value)),
                    hint: self.hint.map(str::to_owned),
                }]
            }

            Kind::Recur => recur_lines(entries.first().copied(), self.key),

            Kind::Duration { .. } => duration_lines(entries.first().copied(), self.key, self.hint),

            // NOTE: an attendee has no line of its own. Its keys are written
            // by attendee_keys, under the header its parent component writes.
            Kind::Attendee { .. } => Vec::new(),
        }
    }

    /// The parameter names this field's projection writes.
    ///
    /// They are the only ones folding a document back owns: every other
    /// parameter on the line belongs to the line.
    fn params(&self) -> &'static [&'static str] {
        match self.kind {
            Kind::Attendee { .. } => &["CN", "ROLE", "PARTSTAT"],
            Kind::Date => &["TZID", "VALUE"],
            _ => &[],
        }
    }

    /// This field's iCalendar content lines, built from a TOML table.
    ///
    /// Empty when absent or blank, so [`crate::ical::TcalComponent::set_lines`]
    /// removes the property. `originals` are its lines in projection order,
    /// each patched rather than rebuilt: a parameter the document omits stays.
    pub(crate) fn content_lines(
        &self,
        source: &dyn TableLike,
        originals: &[String],
    ) -> Vec<String> {
        let Some(item) = source.get(self.key) else {
            return Vec::new();
        };

        // NOTE: each line carries the original it patches onto, which the list
        // field's own spread decides and every other field takes as its first.
        let mut lines: Vec<(Option<&String>, String)> = Vec::new();

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
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
                }
            }

            Kind::Enum => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    let value = value.to_uppercase();
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
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
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
                }
            }

            Kind::List { sep } => {
                if let Some(array) = item.as_array() {
                    let values: Vec<&str> = array
                        .iter()
                        .filter_map(|value| value.as_str())
                        .filter(|value| !value.is_empty())
                        .collect();

                    for (original, items) in spread(&values, originals) {
                        let parts: Vec<String> = items.into_iter().map(escape).collect();
                        let value = parts.join(&sep.to_string());
                        lines.push((original, format!("{}:{}", self.name, value)));
                    }
                }
            }

            Kind::Date => {
                let tz = source
                    .get(&format!("{}-tz", self.key))
                    .and_then(|item| item.as_str())
                    .filter(|value| !value.is_empty());

                if let Some(dtm) = item.as_datetime() {
                    lines.push((originals.first(), toml_date_line(self.name, dtm, tz)));
                } else if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push((originals.first(), date_line(self.name, value, tz)));
                }
            }

            Kind::CalAddress => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    let value = ensure_mailto(value);
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
                }
            }

            Kind::Offset => {
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
                }
            }

            Kind::Attendee { .. } => {
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
                    lines.push((originals.get(lines.len()), line));
                }
            }

            Kind::Recur => {
                if let Some(table) = item.as_table_like()
                    && let Some(rule) = recur_rule(table)
                {
                    lines.push((originals.first(), format!("{}:{}", self.name, rule)));
                }
            }

            Kind::Duration { negative } => {
                if let Some(table) = item.as_table_like()
                    && let Some(value) = duration_value(table, *negative)
                {
                    lines.push((originals.first(), format!("{}:{}", self.name, value)));
                }
            }
        }

        lines
            .into_iter()
            .map(|(original, line)| match original {
                Some(original) => Content(original).rewritten(&line, self.params()),
                None => line,
            })
            .collect()
    }
}

/// Give a list field's items back to the lines they came from.
///
/// An item belongs to the line whose value held it, because a line's
/// parameters describe the items that line carried. Counting them off the
/// front of the array instead hands each line whatever has room, so removing
/// one item relabels every item behind it: `CATEGORIES;LANGUAGE=fr:travail`
/// becomes English and its own line disappears.
///
/// An item no line held fills the room a line lost, in document order, which
/// is how renaming an item rewrites its own line. Whatever is left over shares
/// one new line, and a line left with no items is dropped.
fn spread<'i, 'o>(
    items: &[&'i str],
    originals: &'o [String],
) -> Vec<(Option<&'o String>, Vec<&'i str>)> {
    // NOTE: at most one line leaves nothing to disambiguate, so the items are
    // that line, in the order the document wrote them. Spreading them by
    // ownership instead would write a second line for an added item, which
    // the README's own example says does not happen.
    if originals.len() < 2 {
        return match items.is_empty() {
            true => Vec::new(),
            false => vec![(originals.first(), items.to_vec())],
        };
    }

    let held: Vec<Vec<String>> = originals.iter().map(|line| Content(line).texts()).collect();
    let mut free: Vec<Vec<bool>> = held.iter().map(|texts| vec![true; texts.len()]).collect();
    let mut owners: Vec<Option<usize>> = Vec::with_capacity(items.len());

    for item in items.iter().copied() {
        let mut owner = None;

        for (at, texts) in held.iter().enumerate() {
            let Some(slot) = (0..texts.len()).find(|slot| free[at][*slot] && texts[*slot] == item)
            else {
                continue;
            };

            free[at][slot] = false;
            owner = Some(at);
            break;
        }

        owners.push(owner);
    }

    let mut room: Vec<usize> = free
        .iter()
        .map(|slots| slots.iter().filter(|free| **free).count())
        .collect();
    let mut kept: Vec<Vec<&str>> = held.iter().map(|_| Vec::new()).collect();
    let mut opened = Vec::new();

    for (item, owner) in items.iter().copied().zip(owners) {
        match owner.or_else(|| room.iter().position(|room| *room > 0)) {
            Some(at) => {
                room[at] -= usize::from(owner.is_none());
                kept[at].push(item);
            }
            // NOTE: one line for the lot rather than one line each. Which
            // line's parameters they should have carried is the question
            // several lines make unanswerable, so they carry none, together.
            None => opened.push(item),
        }
    }

    kept.into_iter()
        .zip(originals)
        .filter(|(items, _)| !items.is_empty())
        .map(|(items, original)| (Some(original), items))
        .chain((!opened.is_empty()).then_some((None, opened)))
        .collect()
}
