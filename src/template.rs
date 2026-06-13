//! Projection between a calcard [`ICalendar`] and an ergonomic TOML
//! buffer.
//!
//! [`project`] turns an iCalendar into a fillable TOML form rooted at the
//! `VCALENDAR`: every modeled component is a `[[block]]` (`[[event]]`,
//! `[[todo]]`, ...), with nested children (`[[event.alarm]]`,
//! `[[timezone.standard]]`, ...); a blank calendar lists one example of each
//! type. [`project_with`] narrows that to a chosen set of types: one type
//! flattens as the document root (bare keys, top-level `[[attendee]]` /
//! `[[alarm]]`, no wrapper, the [`project_one`] view), two or more keep the
//! `VCALENDAR` root but show only those types. Known fields are prefilled, the
//! rest listed empty (an empty value means the same as a removed line). Cryptic
//! date-times (`20260613T140000`) become a friendly `2026-06-13 14:00`, with
//! the time zone on its own key.  [`apply`] / [`apply_with`] take the original
//! iCalendar text plus the edited buffer (detecting which shape it is: a
//! component-type key means blocks, otherwise a flat single component) and
//! produce an updated iCalendar, patching only the lines the user changed
//! (through the format-preserving [`crate::edit`]) while keeping every
//! unmodeled property (custom `X-*`, ...) and unmodeled component type
//! byte-for-byte.  A type outside the selection is left untouched, so a
//! filtered view adds to a calendar without dropping what it did not show.
//!
//! `UID` and `DTSTAMP` are intentionally not modeled: they are managed by the
//! app (seeded for new events, preserved otherwise) and cannot be set through
//! the buffer.
//!
//! The buffer is an editing affordance, not an interchange format:
//! `apply` always needs the original iCalendar text, because that is
//! where unmodeled properties and component types live.
//!
//! NOTE: a TOML array-of-tables header captures every key after it, so a
//! component lists its inline fields (including the dotted `recurrence.*`
//! and `duration.*` keys) first, then its sectioned `ATTENDEE` and child
//! components (`[[*.alarm]]`, ...) last.

use calcard::{
    common::PartialDateTime,
    icalendar::{
        ICalendar, ICalendarComponent, ICalendarComponentType, ICalendarEntry,
        ICalendarParameterName,
    },
};
use toml_edit::{DocumentMut, Item, TableLike};

use crate::{
    edit::{self, Container},
    error::{Result, TcalError},
};

/// Project an iCalendar into a fillable TOML form, rooted at the
/// `VCALENDAR`. Every modeled component type is listed as a `[[block]]`
/// (`[[event]]`, `[[todo]]`, ...), with children nested
/// (`[[event.alarm]]`, ...): the actual instances where present, and one
/// empty example for each type that is absent, so the scaffold doubles as
/// documentation of what a calendar can hold. The user keeps what they
/// need; empty blocks are ignored.
pub fn project(ical: &ICalendar) -> String {
    project_filtered(ical, TOP_LEVEL)
}

/// Project a chosen set of component types (by key: `event`, `todo`, ...).
///
/// Nothing selected projects the whole calendar (every type); a single
/// type flattens it as the document root (bare keys, no wrapper); two or
/// more keep the `VCALENDAR` root but show only the chosen types. The
/// unshown types are never lost: [`apply_with`] leaves them untouched.
pub fn project_with(ical: &ICalendar, types: &[String]) -> Result<String> {
    let specs = resolve_specs(types)?;

    Ok(if specs.is_empty() {
        project(ical)
    } else if specs.len() == 1 {
        project_one_spec(ical, specs[0])
    } else {
        project_filtered(ical, &specs)
    })
}

/// Project the given component specs as `[[block]]`s under the `VCALENDAR`
/// root: each instance filled, plus one empty example per absent type.
fn project_filtered(ical: &ICalendar, specs: &[&Spec]) -> String {
    let mut out = String::new();

    out.push_str("# iCalendar as TOML, edited by tcal.\n");
    out.push_str("#\n");
    out.push_str("# Each component is a [[block]]; repeat a block for repeated\n");
    out.push_str("# components, delete one you do not need. Empty fields and empty\n");
    out.push_str("# blocks are ignored. Properties and component types tcal does\n");
    out.push_str("# not model are kept verbatim, not shown here.\n");

    let tops = top_level(ical);

    for spec in specs {
        let instances: Vec<&ICalendarComponent> = tops
            .iter()
            .copied()
            .filter(|component| component.component_type.as_str() == spec.name)
            .collect();

        if instances.is_empty() {
            project_component(&mut out, ical, None, spec, Some(spec.key));
        } else {
            for component in instances {
                project_component(&mut out, ical, Some(component), spec, Some(spec.key));
            }
        }
    }

    out
}

/// Project a single component type flattened as the document root: bare
/// keys at the top level, sections like `[[attendee]]` / `[[alarm]]`, no
/// wrapping header. The first component of that type fills it, or an empty
/// example when there is none.
pub fn project_one(ical: &ICalendar, ty: &str) -> Result<String> {
    let spec = TOP_LEVEL
        .iter()
        .copied()
        .find(|spec| spec.key.eq_ignore_ascii_case(ty))
        .ok_or_else(|| TcalError::UnknownComponent(ty.to_owned()))?;

    Ok(project_one_spec(ical, spec))
}

/// Render one component spec flat at the document root (see [`project_one`]).
fn project_one_spec(ical: &ICalendar, spec: &Spec) -> String {
    let component = top_level(ical)
        .into_iter()
        .find(|component| component.component_type.as_str() == spec.name);

    let mut out = String::new();
    out.push_str("# iCalendar ");
    out.push_str(spec.key);
    out.push_str(" as TOML, edited by tcal.\n");
    out.push_str("#\n");
    out.push_str("# Fill what you need; empty fields are ignored. Other\n");
    out.push_str("# components and properties tcal does not model are kept\n");
    out.push_str("# verbatim, not shown here.\n");

    project_component(&mut out, ical, component, spec, None);
    out
}

/// Resolve component type keys (`event`, `todo`, ...) to their specs, in
/// the given order. An unknown key is an error.
fn resolve_specs(types: &[String]) -> Result<Vec<&'static Spec>> {
    types
        .iter()
        .map(|ty| {
            TOP_LEVEL
                .iter()
                .copied()
                .find(|spec| spec.key.eq_ignore_ascii_case(ty))
                .ok_or_else(|| TcalError::UnknownComponent(ty.clone()))
        })
        .collect()
}

/// Apply an edited TOML buffer onto the original iCalendar text.
///
/// The buffer's shape is detected: a flat event (bare keys, no component
/// header) folds onto the single `VEVENT`; otherwise each `[[block]]`
/// reconciles its component type. Either way, through the format-preserving
/// editor (see [`crate::edit`]) only the lines that actually changed are
/// re-rendered, so unmodeled properties (including the app-managed `UID`
/// and `DTSTAMP`), unmodeled component types, folding, ordering and casing
/// are all kept verbatim. Filled blocks update or add components; cleared
/// blocks remove them.
pub fn apply(original_src: &str, edited_toml: &str) -> Result<String> {
    apply_specs(original_src, edited_toml, &[])
}

/// Apply an edited buffer for a chosen set of component types: only those
/// types are reconciled, so types the filtered view never showed are kept
/// byte-for-byte (editing a `VEVENT` with `--todo` adds a to-do and leaves
/// the event alone). An empty selection reconciles every type (the default,
/// where an emptied block removes its component).
pub fn apply_with(original_src: &str, edited_toml: &str, types: &[String]) -> Result<String> {
    let specs = resolve_specs(types)?;
    apply_specs(original_src, edited_toml, &specs)
}

fn apply_specs(original_src: &str, edited_toml: &str, filter: &[&Spec]) -> Result<String> {
    let doc: DocumentMut = edited_toml.parse().map_err(TcalError::ParseToml)?;

    // A component-type key means the block form; otherwise it is a flat
    // single component whose keys sit at the document top level.
    let blocky = TOP_LEVEL.iter().any(|spec| doc.contains_key(spec.key));

    let mut cal = edit::parse(original_src);

    // Components live inside the VCALENDAR when there is one, else at the
    // document root (a bare component stream).
    if let Some(vcalendar) = cal.component_mut("VCALENDAR") {
        reconcile(vcalendar, &doc, blocky, filter);
    } else {
        reconcile(&mut cal, &doc, blocky, filter);
    }

    Ok(cal.to_string())
}

/// Reconcile `container` against the edited document. In flat mode the
/// whole document is the selected type's table (or a `VEVENT` by default).
/// In block mode each selected type's `[[block]]`s reconcile; an empty
/// selection reconciles every type. Types outside the selection are left
/// untouched, so a filtered view never drops them.
fn reconcile<C: Container>(container: &mut C, doc: &DocumentMut, blocky: bool, filter: &[&Spec]) {
    if !blocky {
        let spec = filter.first().copied().unwrap_or(&VEVENT);
        let count = usize::from(block_has_content(doc.as_table(), spec));
        container.set_component_count(spec.name, count);
        if let Some(component) = container.components_mut(spec.name).next() {
            apply_component(component, doc.as_table(), spec);
        }
        return;
    }

    let specs: Vec<&Spec> = if filter.is_empty() {
        TOP_LEVEL.to_vec()
    } else {
        filter.to_vec()
    };

    for spec in specs {
        let blocks: Vec<&dyn TableLike> = doc
            .get(spec.key)
            .map(tables)
            .unwrap_or_default()
            .into_iter()
            .filter(|table| block_has_content(*table, spec))
            .collect();

        container.set_component_count(spec.name, blocks.len());
        for (component, table) in container.components_mut(spec.name).zip(blocks) {
            apply_component(component, table, spec);
        }
    }
}

/// Rewrite one component's fields and child components from its TOML
/// table, with a minimal diff.
fn apply_component(component: &mut edit::Component, table: &dyn TableLike, spec: &Spec) {
    for field in spec.fields {
        component.set_all(field.name, &field.content_lines(table));
    }

    for child in spec.children {
        let blocks: Vec<&dyn TableLike> = table
            .get(child.key)
            .map(tables)
            .unwrap_or_default()
            .into_iter()
            .filter(|nested| block_has_content(*nested, child))
            .collect();

        component.set_component_count(child.name, blocks.len());
        for (nested, nested_table) in component.components_mut(child.name).zip(blocks) {
            apply_component(nested, nested_table, child);
        }
    }
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
    /// calcard does not unescape (`URL`, `RRULE`, ...).
    Scalar { escape: bool },

    /// Closed RFC 5545 vocabulary (`STATUS`, `CLASS`, `TRANSP`,
    /// `ACTION`): rendered as a bare key, its accepted variants listed
    /// lowercase in the hint, but uppercased to the canonical form on
    /// export.
    Enum,

    /// Integer value, rendered as a bare TOML number (`PRIORITY`,
    /// `PERCENT-COMPLETE`, `REPEAT`).
    Number,

    /// Repeated or multi-valued text, joined on `sep` in the iCalendar
    /// (`CATEGORIES`).
    List { sep: char },

    /// Date or date-time, projected as a friendly `YYYY-MM-DD[ HH:MM]`
    /// plus an adjacent `<key>-tz` time-zone key (`DTSTART`, `DTEND`).
    Date,

    /// Calendar address, projected without its `mailto:` scheme
    /// (`ORGANIZER`).
    CalAddress,

    /// Repeatable attendee with the common `CN` / `ROLE` / `PARTSTAT`
    /// parameters (`ATTENDEE`).
    Attendee,

    /// Recurrence rule (`RRULE`), projected as dotted `recurrence.*` keys
    /// of friendly parts (`frequency`, `interval`, `by-day`, ...), with a
    /// raw-string `recurrence.rule` escape hatch for parts tcal does not
    /// model.
    Recur,

    /// Duration (`DURATION`, or an alarm `TRIGGER` offset), projected as
    /// dotted `<key>.{week,day,hour,min,sec}` magnitude keys. The sign is
    /// implied by context, so `negative` is set for an alarm trigger
    /// (which fires before the event); a value that is not a plain
    /// duration is shown raw as `<key>.raw`.
    Duration { negative: bool },
}

impl Kind {
    /// A bare key (inline, vs a sectioned `[[array]]`). Recurrence and
    /// duration are inline too: their dotted keys flow with the other
    /// fields, so only the attendee array-of-tables is sectioned.
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

/// Shared hint for the friendly date keys: a concrete example date-time.
const DATE_HINT: &str = "2026-06-13 14:30";

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
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "offset-to",
        name: "TZOFFSETTO",
        hint: Some("+0100"),
        kind: Kind::Scalar { escape: false },
    },
    Field {
        key: "date-start",
        name: "DTSTART",
        hint: Some("local start: 1996-10-27 03:00"),
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
struct Spec {
    /// TOML array-of-tables key (e.g. `event`).
    key: &'static str,
    /// iCalendar component name (e.g. `VEVENT`).
    name: &'static str,
    /// Modeled fields, in projection order.
    fields: &'static [Field],
    /// Nested child component specs (e.g. a `VEVENT`'s `VALARM`s).
    children: &'static [&'static Spec],
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

static VEVENT: Spec = Spec {
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
static TOP_LEVEL: &[&Spec] = &[&VEVENT, &VTODO, &VJOURNAL, &VFREEBUSY, &VTIMEZONE];

impl Field {
    /// Render this field into projected lines.
    fn lines(&self, entries: &[&ICalendarEntry]) -> Vec<Line> {
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
                        lhs: format!("{}-tz = {}", self.key, toml_str(tz)),
                        hint: Some("America/New_York; empty for UTC or floating".to_owned()),
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
    /// or blank, so [`crate::edit::Component::set_all`] removes it.
    fn content_lines(&self, source: &dyn TableLike) -> Vec<String> {
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
                if let Some(value) = item.as_str().filter(|value| !value.is_empty()) {
                    let tz = source
                        .get(&format!("{}-tz", self.key))
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

/// The tab width assumed when aligning inline comments. Comments are
/// padded with tabs, not spaces, so their column is a multiple of this.
const TAB_WIDTH: usize = 8;

/// The shared column at which a component's inline `#` comments align: the
/// first tab stop past the widest hinted left side. Padding past the
/// widest line (rather than up to it) means every hinted line reaches the
/// column with at least one tab: one tab too many is fine, one too few
/// would break the column.
fn comment_column<'a>(lines: impl Iterator<Item = &'a Line>) -> usize {
    let widest = lines
        .filter(|line| line.hint.is_some())
        .map(|line| line.lhs.len())
        .max()
        .unwrap_or(0);

    (widest / TAB_WIDTH + 1) * TAB_WIDTH
}

/// Emit projected lines, padding a hinted line with tabs so its `#`
/// comment lands on the shared `column`. A line with an empty left side is
/// a blank separator between field groups.
fn emit_lines(out: &mut String, lines: &[Line], column: usize) {
    for line in lines {
        out.push_str(&line.lhs);

        if let Some(hint) = &line.hint {
            let mut at = line.lhs.len();
            while at < column {
                out.push('\t');
                at = (at / TAB_WIDTH + 1) * TAB_WIDTH;
            }
            out.push_str("# ");
            out.push_str(hint);
        }

        out.push('\n');
    }
}

/// The display group of an inline field, driving the blank line
/// separators: the bare scalar keys form one group and the dates another,
/// while each structured field (every duration or trigger, the recurrence)
/// is its own group, keyed by field key so two adjacent durations stay
/// separated.
fn field_group(field: &Field) -> (u8, &str) {
    match field.kind {
        Kind::Date => (1, ""),
        Kind::Duration { .. } => (2, field.key),
        Kind::Recur => (3, field.key),
        _ => (0, ""),
    }
}

/// Render one attendee block under `header` (e.g. `event.attendee`),
/// filled or empty.
fn attendee_block(lines: &mut Vec<Line>, header: &str, entry: Option<&ICalendarEntry>) {
    lines.push(Line {
        lhs: format!("[[{header}]]"),
        hint: None,
    });

    let display_name = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Cn))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("display-name = {}", toml_str(&display_name)),
        hint: None,
    });

    let value = entry
        .and_then(entry_text)
        .map(strip_mailto)
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("value = {}", toml_str(value)),
        hint: Some("email address".to_owned()),
    });

    let role = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Role))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("role = {}", toml_str(&role)),
        hint: Some("chair, req-participant, opt-participant, non-participant".to_owned()),
    });

    let status = entry
        .and_then(|entry| param(entry, &ICalendarParameterName::Partstat))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("status = {}", toml_str(&status)),
        hint: Some("needs-action, accepted, declined, tentative, delegated".to_owned()),
    });
}

/// The top-level components of an iCalendar (the `VCALENDAR`'s children),
/// or the lone component of a bare stream.
fn top_level(ical: &ICalendar) -> Vec<&ICalendarComponent> {
    let Some(root) = ical.components.first() else {
        return Vec::new();
    };

    if root.component_type == ICalendarComponentType::VCalendar {
        root.component_ids
            .iter()
            .filter_map(|id| ical.components.get(*id as usize))
            .collect()
    } else {
        vec![root]
    }
}

/// The entries of a component matching a field's name (empty when the
/// component is absent, for example blocks).
fn entries_for<'a>(
    component: Option<&'a ICalendarComponent>,
    field: &Field,
) -> Vec<&'a ICalendarEntry> {
    component
        .map(|component| {
            component
                .entries
                .iter()
                .filter(|entry| entry.name.as_str() == field.name)
                .collect()
        })
        .unwrap_or_default()
}

/// The child components of `component` matching a child spec's type.
fn child_components<'a>(
    ical: &'a ICalendar,
    component: Option<&ICalendarComponent>,
    child: &Spec,
) -> Vec<&'a ICalendarComponent> {
    component
        .map(|component| {
            component
                .component_ids
                .iter()
                .filter_map(|id| ical.components.get(*id as usize))
                .filter(|nested| nested.component_type.as_str() == child.name)
                .collect()
        })
        .unwrap_or_default()
}

/// Render a component as a `[[prefix]]` block: its simple fields as one
/// aligned key block, its attendee fields and child components as nested
/// `[[prefix.key]]` blocks, recursively.
fn project_component(
    out: &mut String,
    ical: &ICalendar,
    component: Option<&ICalendarComponent>,
    spec: &Spec,
    prefix: Option<&str>,
) {
    // `None` is the flat top-level event: no `[[block]]` header, and its
    // sections sit at the top level (`[[attendee]]`, not `[[x.attendee]]`).
    if let Some(prefix) = prefix {
        out.push('\n');
        out.push_str("[[");
        out.push_str(prefix);
        out.push_str("]]\n");
    }

    let mut simple = Vec::new();
    let mut group = None;
    for field in spec.fields.iter().filter(|field| field.kind.is_simple()) {
        let key = field_group(field);
        if group.is_some_and(|previous| previous != key) {
            simple.push(Line {
                lhs: String::new(),
                hint: None,
            });
        }
        group = Some(key);
        simple.extend(field.lines(&entries_for(component, field)));
    }

    let sections: Vec<Vec<Line>> = spec
        .fields
        .iter()
        .filter(|field| !field.kind.is_simple())
        .map(|field| {
            let entries = entries_for(component, field);
            attendee_section(&entries, &section_header(prefix, field.key))
        })
        .collect();

    // One column for the whole component, so every comment aligns at the
    // same level: across the field groups and the attendee section alike.
    let column = comment_column(simple.iter().chain(sections.iter().flatten()));

    emit_lines(out, &simple, column);
    for section in &sections {
        out.push('\n');
        emit_lines(out, section, column);
    }

    for child in spec.children {
        let nested = child_components(ical, component, child);
        let child_prefix = section_header(prefix, child.key);

        if nested.is_empty() {
            project_component(out, ical, None, child, Some(&child_prefix));
        } else {
            for kid in nested {
                project_component(out, ical, Some(kid), child, Some(&child_prefix));
            }
        }
    }
}

/// The TOML header for a section/child `key` under an optional parent
/// `prefix`: `"key"` at the top level (flat), else `"prefix.key"`.
fn section_header(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}.{key}"),
        None => key.to_owned(),
    }
}

/// Render an attendee field as `[[header]]` blocks, one per entry, or a
/// single empty example.
fn attendee_section(entries: &[&ICalendarEntry], header: &str) -> Vec<Line> {
    let mut lines = Vec::new();

    if entries.is_empty() {
        attendee_block(&mut lines, header, None);
    } else {
        for entry in entries.iter().copied() {
            attendee_block(&mut lines, header, Some(entry));
        }
    }

    lines
}

/// Render a recurrence rule as dotted `prefix.*` keys of friendly parts
/// (`prefix` is `recurrence`, or `event.recurrence` etc. is irrelevant:
/// dotted keys are relative to the current table). A rule that uses parts
/// tcal does not model (`BYHOUR`, `RSCALE`, ...) is shown instead as a
/// single raw `prefix.rule` key, so it round-trips intact.
fn recur_lines(entry: Option<&ICalendarEntry>, prefix: &str) -> Vec<Line> {
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
        recur_str_list_line(
            &format!("{prefix}.by-day"),
            get("BYDAY"),
            "mo, tu, we, th, fr, sa, su; with an ordinal like -1su, 2mo",
        ),
        recur_int_list_line(&format!("{prefix}.by-month"), get("BYMONTH"), "1 to 12"),
        recur_int_list_line(
            &format!("{prefix}.by-month-day"),
            get("BYMONTHDAY"),
            "1 to 31, negative counts from the end (-1 = last)",
        ),
        recur_int_list_line(
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

/// A dotted integer key (a recurrence or duration part): a bare number
/// when set, an empty string (ignored on apply) otherwise, with an
/// optional hint.
fn int_line(key: &str, value: Option<i64>, hint: Option<&str>) -> Line {
    let lhs = match value {
        Some(value) => format!("{key} = {value}"),
        None => format!("{key} = \"\""),
    };
    Line {
        lhs,
        hint: hint.map(str::to_owned),
    }
}

/// A recurrence string-list key (e.g. `byday`), lowercased for display.
fn recur_str_list_line(key: &str, value: Option<&str>, hint: &str) -> Line {
    let items: Vec<String> = value
        .map(|value| value.split(',').map(str::to_lowercase).collect())
        .unwrap_or_default();
    Line {
        lhs: format!("{key} = {}", toml_array(&items)),
        hint: Some(hint.to_owned()),
    }
}

/// A recurrence integer-list key (e.g. `bymonthday`).
fn recur_int_list_line(key: &str, value: Option<&str>, hint: &str) -> Line {
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

/// Whether a TOML block carries any modeled value, i.e. is a real
/// component rather than an empty example placeholder.
fn block_has_content(table: &dyn TableLike, spec: &Spec) -> bool {
    spec.fields
        .iter()
        .any(|field| !field.content_lines(table).is_empty())
        || spec.children.iter().any(|child| {
            table
                .get(child.key)
                .map(tables)
                .unwrap_or_default()
                .iter()
                .any(|nested| block_has_content(*nested, child))
        })
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
        out.push_str(&format!(":{second:02}"));
    }
    if matches!((dt.tz_hour, dt.tz_minute), (Some(0), Some(0))) {
        out.push_str(" UTC");
    }

    out
}

/// The `RRULE` tokens tcal projects as structured keys, in calcard's
/// canonical serialization order. A rule using any other token (`BYHOUR`,
/// `RSCALE`, ...) is shown as a raw string so it round-trips losslessly.
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

/// Split an `RRULE` value (`FREQ=WEEKLY;INTERVAL=2`) into uppercased
/// token names paired with their raw value.
fn parse_rrule(rule: &str) -> Vec<(String, String)> {
    rule.split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(name, value)| (name.to_uppercase(), value.to_owned()))
        .collect()
}

/// Assemble an `RRULE` value from a `[recurrence]` table, emitting the
/// tokens in [`RECUR_KEYS`] order so an untouched rule round-trips
/// byte-for-byte. A non-empty `rule` key short-circuits to its raw value.
fn recur_rule(table: &dyn TableLike) -> Option<String> {
    if let Some(rule) = recur_text(table, "rule") {
        return Some(rule);
    }

    let freq = recur_text(table, "frequency")?;
    let mut parts = vec![format!("FREQ={}", freq.to_uppercase())];

    if let Some(until) = recur_text(table, "until") {
        parts.push(format!("UNTIL={}", friendly_to_ical(&until)));
    }
    if let Some(count) = recur_int(table, "count") {
        parts.push(format!("COUNT={count}"));
    }
    if let Some(interval) = recur_int(table, "interval") {
        parts.push(format!("INTERVAL={interval}"));
    }

    let byday = recur_str_list(table, "by-day");
    if !byday.is_empty() {
        parts.push(format!("BYDAY={}", byday.join(",")));
    }

    for (key, token) in [
        ("by-month-day", "BYMONTHDAY"),
        ("by-month", "BYMONTH"),
        ("by-position", "BYSETPOS"),
    ] {
        let values = recur_int_list(table, key);
        if !values.is_empty() {
            parts.push(format!("{token}={}", join_ints(&values)));
        }
    }

    if let Some(wkst) = recur_text(table, "week-start") {
        parts.push(format!("WKST={}", wkst.to_uppercase()));
    }

    Some(parts.join(";"))
}

/// A non-empty string from a recurrence table key.
fn recur_text(table: &dyn TableLike, key: &str) -> Option<String> {
    table
        .get(key)
        .and_then(|item| item.as_str())
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

/// An integer from a recurrence table key, accepting a bare number or a
/// numeric string.
fn recur_int(table: &dyn TableLike, key: &str) -> Option<i64> {
    let item = table.get(key)?;
    item.as_integer()
        .or_else(|| item.as_str().and_then(|value| value.trim().parse().ok()))
}

/// An uppercased string list (e.g. `byday`) from a recurrence table key.
fn recur_str_list(table: &dyn TableLike, key: &str) -> Vec<String> {
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

/// An integer list (e.g. `bymonthday`) from a recurrence table key,
/// accepting bare numbers or numeric strings.
fn recur_int_list(table: &dyn TableLike, key: &str) -> Vec<i64> {
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

/// Render an `RRULE` `UNTIL` value (`20261231T000000Z`) as a friendly
/// `YYYY-MM-DD [HH:MM[:SS]] [UTC]`, passing it through verbatim when it is
/// not in the expected digit form.
fn ical_to_friendly(raw: &str) -> String {
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

/// Convert a friendly date back to the `RRULE` `UNTIL` digit form,
/// passing it through verbatim when it is not friendly.
fn friendly_to_ical(value: &str) -> String {
    match parse_friendly_date(value) {
        Some((date, None, _)) => date,
        Some((date, Some(time), true)) => format!("{date}T{time}Z"),
        Some((date, Some(time), false)) => format!("{date}T{time}"),
        None => value.to_owned(),
    }
}

/// Render a duration value as dotted `<prefix>.{week,day,hour,min,sec}`
/// magnitude keys, the field's hint on the leading line. The sign is
/// implied by context (a trigger fires before the event), so the parts are
/// unsigned. A value that is not a plain duration (an absolute date-time
/// trigger) is shown raw as `<prefix>.raw`, so it round-trips intact.
fn duration_lines(entry: Option<&ICalendarEntry>, prefix: &str, hint: Option<&str>) -> Vec<Line> {
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

/// Parse an iCalendar duration (`P1DT2H30M`, `PT15M`, `P2W`, optionally
/// signed) into unsigned magnitudes `(week, day, hour, minute, second)`.
/// `None` when the value is not a duration, so the caller keeps it raw.
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

/// Split a duration segment (`1D`, `2H30M`) into `(number, unit)` pairs.
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

/// Assemble an iCalendar duration from a duration table's
/// `week/day/hour/min/sec` parts (or a raw `raw` escape hatch), prefixing
/// `-` when `negative`. A lone week stays `P<n>W`; weeks otherwise fold
/// into days. `None` when no part is set.
fn duration_value(table: &dyn TableLike, negative: bool) -> Option<String> {
    if let Some(raw) = recur_text(table, "raw") {
        return Some(raw);
    }

    let week = recur_int(table, "week").unwrap_or(0);
    let day = recur_int(table, "day").unwrap_or(0);
    let hour = recur_int(table, "hour").unwrap_or(0);
    let minute = recur_int(table, "min").unwrap_or(0);
    let second = recur_int(table, "sec").unwrap_or(0);

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

/// Append `;NAME=value` to `line` when the table entry is non-empty,
/// quoting the value when it carries a parameter delimiter. `upper`
/// uppercases the value, for closed vocabularies (`ROLE`, `PARTSTAT`).
fn push_param(line: &mut String, name: &str, item: Option<&Item>, upper: bool) {
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

/// Render an integer value as a bare TOML number, an empty string (which
/// [`Kind::Number`] ignores) when blank, or a quoted fallback when it is
/// not a plain integer.
fn toml_number(value: &str) -> String {
    if value.is_empty() {
        "\"\"".to_owned()
    } else if value.parse::<i64>().is_ok() {
        value.to_owned()
    } else {
        toml_str(value)
    }
}

/// Render strings as a TOML array.
fn toml_array<S: AsRef<str>>(items: &[S]) -> String {
    let mut array = toml_edit::Array::new();

    for item in items {
        array.push(item.as_ref());
    }

    array.to_string().trim().to_string()
}

/// Render integers as a TOML array.
fn toml_int_array(items: &[i64]) -> String {
    let mut array = toml_edit::Array::new();

    for item in items {
        array.push(*item);
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

        // The default root is the VCALENDAR: components are [[blocks]].
        assert!(toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
        assert!(toml.contains("date-start = \"2026-06-13 14:00\""));
        assert!(toml.contains("date-start-tz = \"America/New_York\""));
        assert!(toml.contains("location = \"Room 1\""));
        assert!(toml.contains("[[event.attendee]]"));
        assert!(toml.contains("value = \"jane@example.com\""));
        assert!(toml.contains("display-name = \"Jane Doe\""));
        assert!(toml.contains("[[event.alarm]]"));
        assert!(toml.contains("action = \"DISPLAY\""));
        // The alarm trigger is structured, its sign implied (fires before).
        assert!(toml.contains("trigger.min = 15"));
        // Every other component type still appears as an empty example,
        // even though the calendar holds only an event.
        assert!(toml.contains("[[todo]]"));
        assert!(toml.contains("[[journal]]"));
        assert!(toml.contains("[[free-busy]]"));
        assert!(toml.contains("[[timezone]]"));
        // Unmodeled data never appears in the scaffold.
        assert!(!toml.contains("keep me verbatim"));
        assert!(!toml.contains("DTSTAMP"));
    }

    #[test]
    fn blank_project_shows_every_component_type() {
        // A blank calendar is a starting menu of every modeled component.
        let toml = super::project(&Default::default());

        for block in [
            "[[event]]",
            "[[event.alarm]]",
            "[[todo]]",
            "[[journal]]",
            "[[free-busy]]",
            "[[timezone]]",
            "[[timezone.standard]]",
        ] {
            assert!(toml.contains(block), "missing {block}");
        }
    }

    #[test]
    fn project_one_flattens_a_component() {
        // A single chosen type, flat at the root, no wrapper.
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project_one(&ical, "event").unwrap();

        assert!(!toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
        assert!(toml.contains("[[attendee]]"));
        assert!(toml.contains("[[alarm]]"));

        // An unknown component type is an error.
        assert!(super::project_one(&ical, "nope").is_err());
    }

    #[test]
    fn project_one_round_trips_flat() {
        // A flat buffer (no component key) folds back onto the event.
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project_one(&ical, "event").unwrap();

        assert_eq!(super::apply(SAMPLE, &toml).unwrap(), SAMPLE);
    }

    #[test]
    fn richer_calendar_projects_blocks() {
        // Several events project as several [[blocks]], children nested.
        let src = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\nSUMMARY:a\r\nEND:VEVENT\r\n\
            BEGIN:VEVENT\r\nSUMMARY:b\r\nEND:VEVENT\r\n\
            END:VCALENDAR\r\n";
        let ical = ical::parse(src).unwrap();
        let toml = super::project(&ical);

        assert_eq!(toml.lines().filter(|line| *line == "[[event]]").count(), 2);
        assert!(toml.contains("[[event.alarm]]"));
    }

    #[test]
    fn blank_project_layout() {
        let toml = super::project(&Default::default());

        // Within the event block: summary leads, start before end,
        // the description bare key before the nested attendee block.
        assert!(!toml.contains("uid"));
        assert!(toml.find("summary =").unwrap() < toml.find("date-start =").unwrap());
        assert!(toml.find("date-start =").unwrap() < toml.find("date-end =").unwrap());
        assert!(toml.find("description =").unwrap() < toml.find("[[event.attendee]]").unwrap());

        // Empty, uncommented fields; description as a plain empty string.
        assert!(toml.contains("summary = \"\""));
        assert!(toml.contains("description = \"\""));
        assert!(!toml.contains("#summary"));

        // No "required" flag (omitting a field drops it); hints carry no
        // "e.g." prefix, list enum variants lowercase, and show formats
        // (dates as a concrete example).
        assert!(!toml.contains("# required"));
        assert!(toml.contains("# 2026-06-13 14:30"));
        assert!(toml.contains("# display, email, audio"));
        assert!(toml.contains("# confirmed, tentative, cancelled"));
        assert!(!toml.contains("e.g."));

        // Recurrence is inlined as dotted `recurrence.*` keys.
        assert!(toml.contains("recurrence.frequency = \"\""));
        assert!(!toml.contains("[event.recurrence]"));
        assert!(
            toml.find("recurrence.frequency").unwrap() < toml.find("[[event.attendee]]").unwrap()
        );
    }

    #[test]
    fn hints_are_tab_aligned() {
        let toml = super::project(&Default::default());

        // Every inline hint is separated from its value by a tab, so the
        // comment lands at a tab stop instead of a far, space-padded
        // column. No hinted key line carries a run of padding spaces.
        let hinted: Vec<&str> = toml
            .lines()
            .filter(|line| line.contains('=') && line.contains('#'))
            .collect();
        assert!(!hinted.is_empty());

        for line in hinted {
            assert!(line.contains("\t#"), "not tab-aligned: {line:?}");
            let before = &line[..line.find('#').unwrap()];
            assert!(!before.contains("  "), "space padded: {line:?}");
        }
    }

    #[test]
    fn apply_projection_is_a_no_op() {
        // Projecting then applying an untouched buffer reproduces the
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
    fn apply_edits_an_existing_alarm() {
        // Editing the alarm's structured trigger touches only that line.
        let toml = super::project(&ical::parse(SAMPLE).unwrap())
            .replace("trigger.min = 15", "trigger.min = 30");

        let out = super::apply(SAMPLE, &toml).unwrap();

        assert_eq!(out, SAMPLE.replace("TRIGGER:-PT15M", "TRIGGER:-PT30M"));
    }

    #[test]
    fn apply_roundtrip_preserves_unmodeled() {
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project(&ical);

        let out = super::apply(SAMPLE, &toml).unwrap();

        assert!(out.contains("SUMMARY:Team sync"));
        assert!(out.contains("DTSTART;TZID=America/New_York:20260613T140000"));
        assert!(out.contains("mailto:jane@example.com"));
        // Unmodeled property and app-managed bookkeeping survive verbatim.
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
        assert!(out.contains("DTSTAMP:20260101T000000Z"));
        assert!(out.contains("TRIGGER:-PT15M"));
    }

    #[test]
    fn uid_is_hidden_and_app_managed() {
        let ical = ical::parse(SAMPLE).unwrap();

        // Hidden from the form.
        let toml = super::project(&ical);
        assert!(!toml.contains("uid"));

        // Preserved on round-trip, and not overridable from the buffer.
        let edited = "[[event]]\nsummary = \"Team sync\"\nuid = \"hacked\"\n";
        let out = super::apply(SAMPLE, edited).unwrap();
        assert!(out.contains("UID:abc@example"));
        assert!(!out.contains("hacked"));
    }

    #[test]
    fn apply_edits_modeled_field() {
        let edited = "[[event]]\nsummary = \"New title\"\n";

        let out = super::apply(SAMPLE, edited).unwrap();

        assert!(out.contains("SUMMARY:New title"));
        assert!(!out.contains("Team sync"));
        // The event is kept, so its unmodeled property stays.
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_renders_all_day_and_utc_dates() {
        let all_day = super::apply(SAMPLE, "[[event]]\ndate-start = \"2026-12-25\"\n").unwrap();
        assert!(all_day.contains("DTSTART;VALUE=DATE:20261225"));

        let utc =
            super::apply(SAMPLE, "[[event]]\ndate-start = \"2026-06-13 14:00 UTC\"\n").unwrap();
        assert!(utc.contains("DTSTART:20260613T140000Z"));

        let zoned = super::apply(
            SAMPLE,
            "[[event]]\ndate-start = \"2026-06-13 09:30\"\ndate-start-tz = \"Europe/Paris\"\n",
        )
        .unwrap();
        assert!(zoned.contains("DTSTART;TZID=Europe/Paris:20260613T093000"));
    }

    #[test]
    fn apply_adds_an_alarm() {
        // An event with no alarm gains one from a filled nested block.
        let src = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:Solo\r\n\
            END:VEVENT\r\n\
            END:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"Solo\"\n\n\
            [[event.alarm]]\naction = \"DISPLAY\"\ntrigger.min = 10\n";

        let out = super::apply(src, edited).unwrap();

        assert!(out.contains("BEGIN:VEVENT\r\nSUMMARY:Solo\r\nBEGIN:VALARM\r\n"));
        assert!(out.contains("ACTION:DISPLAY\r\n"));
        assert!(out.contains("TRIGGER:-PT10M\r\n"));
        assert!(out.contains("END:VALARM\r\nEND:VEVENT\r\n"));
    }

    #[test]
    fn apply_removes_an_alarm() {
        // An event block with no nested alarm drops the alarm, while the
        // event and its unmodeled property stay.
        let out = super::apply(SAMPLE, "[[event]]\nsummary = \"Team sync\"\n").unwrap();

        assert!(!out.contains("BEGIN:VALARM"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_empty_buffer_removes_modeled_components() {
        // No blocks means no modeled components; the VCALENDAR wrapper and
        // its own properties remain.
        let out = super::apply(SAMPLE, "").unwrap();

        assert!(!out.contains("BEGIN:VEVENT"));
        assert!(out.contains("BEGIN:VCALENDAR"));
        assert!(out.contains("VERSION:2.0"));
        assert!(out.contains("PRODID:-//Test//EN"));
    }

    #[test]
    fn projects_and_edits_multiple_events() {
        let src = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:first\r\n\
            END:VEVENT\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:second\r\n\
            END:VEVENT\r\n\
            END:VCALENDAR\r\n";
        let ical = ical::parse(src).unwrap();
        let toml = super::project(&ical);

        // Two events project as two blocks.
        assert_eq!(toml.matches("[[event]]").count(), 2);

        // Editing the second leaves the first byte-for-byte.
        let edited = toml.replace("second", "2nd");
        let out = super::apply(src, &edited).unwrap();
        assert_eq!(out, src.replace("SUMMARY:second", "SUMMARY:2nd"));
    }

    #[test]
    fn apply_adds_a_todo() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[todo]]\nsummary = \"Submit report\"\ndate-due = \"2026-06-20 17:00\"\n";

        let out = super::apply(src, edited).unwrap();

        assert!(out.contains("BEGIN:VTODO\r\n"));
        assert!(out.contains("SUMMARY:Submit report\r\n"));
        assert!(out.contains("DUE:20260620T170000\r\n"));
        assert!(out.contains("END:VTODO\r\n"));
    }

    #[test]
    fn apply_uppercases_enum_values() {
        // Enum properties and the attendee role/partstat parameters are
        // listed lowercase but exported in their canonical uppercase form.
        let edited = "[[event]]\nsummary = \"Team sync\"\nstatus = \"confirmed\"\n\n\
            [[event.attendee]]\nvalue = \"jane@example.com\"\n\
            role = \"req-participant\"\nstatus = \"accepted\"\n";

        let out = super::apply(SAMPLE, edited).unwrap();

        assert!(out.contains("STATUS:CONFIRMED"));
        assert!(out.contains("ROLE=REQ-PARTICIPANT"));
        assert!(out.contains("PARTSTAT=ACCEPTED"));
        // Free-text values are not touched.
        assert!(out.contains("SUMMARY:Team sync"));
    }

    const RECUR_SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\n\
        SUMMARY:Standup\r\n\
        RRULE:FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE,FR\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    #[test]
    fn recurrence_projects_structured_parts() {
        let toml = super::project(&ical::parse(RECUR_SAMPLE).unwrap());

        assert!(toml.contains("recurrence.frequency = \"weekly\""));
        assert!(toml.contains("recurrence.interval = 2"));
        assert!(toml.contains("recurrence.by-day = [\"mo\", \"we\", \"fr\"]"));
    }

    #[test]
    fn recurrence_round_trips() {
        // A canonical rule survives project -> apply byte-for-byte.
        let toml = super::project(&ical::parse(RECUR_SAMPLE).unwrap());

        assert_eq!(super::apply(RECUR_SAMPLE, &toml).unwrap(), RECUR_SAMPLE);
    }

    #[test]
    fn recurrence_assembles_from_parts() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"x\"\n\n\
            [event.recurrence]\nfrequency = \"monthly\"\nby-month-day = [-1]\n";

        let out = super::apply(src, edited).unwrap();

        assert!(out.contains("RRULE:FREQ=MONTHLY;BYMONTHDAY=-1\r\n"));
    }

    #[test]
    fn recurrence_until_is_friendly() {
        // UNTIL projects as a friendly date and reassembles to digits.
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            RRULE:FREQ=DAILY;UNTIL=20261231T235900Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains("recurrence.until = \"2026-12-31 23:59 UTC\""));
        assert_eq!(super::apply(src, &toml).unwrap(), src);
    }

    #[test]
    fn recurrence_raw_fallback_for_unmodeled_parts() {
        // A rule with a part tcal does not model (BYHOUR) is shown raw and
        // round-trips intact rather than being silently dropped.
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            RRULE:FREQ=DAILY;BYHOUR=9\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains("recurrence.rule = \"FREQ=DAILY;BYHOUR=9\""));
        assert_eq!(super::apply(src, &toml).unwrap(), src);
    }

    const DURATION_SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\n\
        SUMMARY:Workshop\r\n\
        DURATION:P1DT2H30M\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    #[test]
    fn duration_projects_structured_parts() {
        let toml = super::project(&ical::parse(DURATION_SAMPLE).unwrap());

        assert!(toml.contains("duration.day = 1"));
        assert!(toml.contains("duration.hour = 2"));
        assert!(toml.contains("duration.min = 30"));
        assert!(toml.contains("duration.week = \"\""));
    }

    #[test]
    fn duration_round_trips() {
        let toml = super::project(&ical::parse(DURATION_SAMPLE).unwrap());

        assert_eq!(
            super::apply(DURATION_SAMPLE, &toml).unwrap(),
            DURATION_SAMPLE
        );
    }

    #[test]
    fn duration_assembles_with_implied_sign() {
        // A bare duration is positive; an alarm trigger is negative, its
        // sign assumed from context rather than typed.
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"x\"\nduration.hour = 1\nduration.min = 30\n\n\
            [[event.alarm]]\naction = \"DISPLAY\"\ntrigger.min = 15\n";

        let out = super::apply(src, edited).unwrap();

        assert!(out.contains("DURATION:PT1H30M\r\n"));
        assert!(out.contains("TRIGGER:-PT15M\r\n"));
    }

    #[test]
    fn duration_lone_week_stays_weekly() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let out = super::apply(src, "[[event]]\nsummary = \"x\"\nduration.week = 2\n").unwrap();

        assert!(out.contains("DURATION:P2W\r\n"));
    }

    #[test]
    fn trigger_raw_fallback_for_date_time() {
        // An absolute date-time trigger is not a plain duration, so it
        // falls back to a raw key and is kept (not silently dropped) on
        // apply, carrying the value the reader parsed.
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            BEGIN:VALARM\r\nACTION:DISPLAY\r\n\
            TRIGGER;VALUE=DATE-TIME:20260101T120000Z\r\n\
            END:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains("trigger.raw = "));
        let out = super::apply(src, &toml).unwrap();
        assert!(out.contains("TRIGGER:2026-01-01T12:00:00Z"));
    }

    #[test]
    fn attendee_display_name_leads() {
        // The display name is the first key of an attendee block, and maps
        // back to the CN parameter on apply.
        let toml = super::project(&ical::parse(SAMPLE).unwrap());
        let block = toml.split("[[event.attendee]]").nth(1).unwrap();
        assert!(block.find("display-name =").unwrap() < block.find("value =").unwrap());

        let edited = "[[event]]\nsummary = \"x\"\n\n\
            [[event.attendee]]\ndisplay-name = \"Jane Doe\"\nvalue = \"jane@example.com\"\n";
        let out = super::apply(SAMPLE, edited).unwrap();
        assert!(out.contains("CN=Jane Doe"));
    }

    #[test]
    fn alarm_separates_trigger_and_duration() {
        // The two structured durations are their own groups: a blank line
        // sits between the last trigger part and the first duration part.
        let toml = super::project(&Default::default());
        assert!(toml.contains("trigger.sec = \"\"\n\nduration.week = \"\""));
    }

    #[test]
    fn project_with_no_flags_shows_all() {
        let ical = ical::parse(SAMPLE).unwrap();
        assert_eq!(
            super::project_with(&ical, &[]).unwrap(),
            super::project(&ical)
        );
    }

    #[test]
    fn project_with_one_flag_flattens() {
        // A single selected type flattens as the root, like project_one.
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project_with(&ical, &["event".to_owned()]).unwrap();

        assert!(!toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
    }

    #[test]
    fn project_with_many_flags_filters_blocks() {
        // Two or more keep the VCALENDAR root but show only those types.
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project_with(&ical, &["event".to_owned(), "todo".to_owned()]).unwrap();

        assert!(toml.contains("[[event]]"));
        assert!(toml.contains("[[todo]]"));
        assert!(!toml.contains("[[journal]]"));
        assert!(!toml.contains("[[timezone]]"));
    }

    #[test]
    fn apply_with_filter_preserves_unselected_block() {
        // Editing the event source filtered to a filled todo block adds the
        // to-do and leaves the event (and its unmodeled property) verbatim.
        let edited = "[[todo]]\nsummary = \"Submit report\"\n";
        let out = super::apply_with(SAMPLE, edited, &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VTODO"));
        assert!(out.contains("SUMMARY:Submit report"));
        assert!(out.contains("SUMMARY:Team sync"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_with_filter_does_not_remove_unselected() {
        // An empty todo block under a todo filter must NOT drop the event:
        // only selected types are reconciled.
        let out =
            super::apply_with(SAMPLE, "[[todo]]\nsummary = \"\"\n", &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VEVENT"));
        assert!(out.contains("SUMMARY:Team sync"));
        assert!(!out.contains("BEGIN:VTODO"));
    }

    #[test]
    fn apply_with_flat_one_type_merges() {
        // --todo on an event source: a flat todo buffer adds a VTODO and
        // keeps the VEVENT (the headline merge behaviour).
        let ical = ical::parse(SAMPLE).unwrap();
        let toml = super::project_with(&ical, &["todo".to_owned()]).unwrap();
        let filled = toml.replace("summary = \"\"", "summary = \"My task\"");

        let out = super::apply_with(SAMPLE, &filled, &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VTODO"));
        assert!(out.contains("SUMMARY:My task"));
        assert!(out.contains("SUMMARY:Team sync"));
    }

    #[test]
    fn fields_are_grouped() {
        // The fields cluster by shape: the bare scalar keys (summary and
        // description leading), then the dates, the duration, and the
        // recurrence, each its own group, with the attendee section last.
        let toml = super::project(&Default::default());
        let at = |needle: &str| toml.find(needle).unwrap();

        // Headline scalars lead the scalar group.
        assert!(at("summary =") < at("description ="));
        assert!(at("description =") < at("categories ="));
        // All scalars precede the dates, the dates the duration, the
        // duration the recurrence, the recurrence the attendee section.
        assert!(at("transparency =") < at("date-start ="));
        assert!(at("date-start =") < at("date-end ="));
        assert!(at("date-end =") < at("duration.week ="));
        assert!(at("duration.sec =") < at("recurrence.frequency ="));
        assert!(at("recurrence.week-start =") < at("[[event.attendee]]"));
    }
}
