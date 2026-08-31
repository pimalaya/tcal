//! # Projection
//!
//! The two directions between a calendar and the ergonomic TOML form a reader
//! edits: projecting one out, and folding an edited one back.
//!
//! [`TcalTemplate`] carries the calendar and the component types it shows, so the
//! tree it projects is the tree it patches. Nothing selected shows every
//! modelled type as a `[[block]]`, one type flattens at the document root, and
//! two or more keep the `VCALENDAR` root while showing only those.
//!
//! A type left out is never lost: a fold-back reconciles the selected ones and
//! keeps every other byte, so editing an event with `--todo` adds a to-do and
//! leaves the event alone.
//!
//! The modelled vocabulary lives in the model submodule, and the values with a
//! shape of their own in datetime, duration and recurrence. `UID` and `DTSTAMP`
//! are app-managed, not modelled.

mod datetime;
mod duration;
pub(crate) mod line;
pub(crate) mod model;
pub(crate) mod patch;
mod recurrence;
pub(crate) mod toml;

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec::Vec,
};

use ical::tree::{codec::mode::Escaper, cst::IcalCst, line::IcalLine};
use toml_edit::{DocumentMut, TableLike};

use crate::{
    error::{TcalError, TcalResult},
    ical::{TcalCalendar, TcalComponent, TcalContainer, TcalProp},
    template::{
        line::{Line, comment_column, emit_lines},
        model::{Field, Kind, Spec, TOP_LEVEL, VEVENT},
        patch::strip_mailto,
        toml::{tables, toml_str},
    },
};

/// An iCalendar stream and the TOML form it is edited through.
pub struct TcalTemplate<'a> {
    /// The calendar the form shows.
    pub calendar: TcalCalendar<'a>,
    /// The component types the form is narrowed to, every one when empty.
    ///
    /// Set through [`TcalTemplate::with_types`], which resolves the keys a reader
    /// names into the specs behind them.
    pub(crate) types: Vec<&'static Spec>,
}

impl<'a> TcalTemplate<'a> {
    /// Read an iCalendar stream as the form it will be edited through.
    pub fn parse(source: &'a str) -> TcalResult<Self> {
        Ok(Self {
            calendar: TcalCalendar::parse(source)?,
            types: Vec::new(),
        })
    }

    /// Narrow the form to the given component type keys (`event`, `todo`, ...).
    ///
    /// They keep the given order, and an unknown key is an error.
    pub fn with_types(mut self, keys: &[String]) -> TcalResult<Self> {
        self.types = keys
            .iter()
            .map(|key| {
                TOP_LEVEL
                    .iter()
                    .copied()
                    .find(|spec| spec.key.eq_ignore_ascii_case(key))
                    .ok_or_else(|| TcalError::UnknownComponent(key.clone()))
            })
            .collect::<TcalResult<Vec<_>>>()?;

        Ok(self)
    }

    /// Project the calendar into a fillable TOML form.
    ///
    /// Each instance is filled, plus one empty example per absent type.
    pub fn project(&self) -> String {
        match self.types.as_slice() {
            [] => self.project_blocks(TOP_LEVEL),
            [spec] => self.project_flat(spec),
            specs => self.project_blocks(specs),
        }
    }

    /// Fold an edited form back onto the calendar it was projected from.
    ///
    /// Only changed lines are re-rendered, everything unmodelled staying byte
    /// for byte. A filled block updates or adds a component, and a cleared one
    /// removes it.
    pub fn apply(&self, edited: &str) -> TcalResult<String> {
        let doc: DocumentMut = edited.parse().map_err(TcalError::ParseToml)?;

        // NOTE: a component-type key means the block form, otherwise it is a
        // flat single component whose keys sit at the document top level.
        let blocky = TOP_LEVEL.iter().any(|spec| doc.contains_key(spec.key));

        let mut calendar = self.calendar.clone();
        let escaper = calendar.escaper();

        // NOTE: components live inside the VCALENDAR when there is one, else
        // beside it in the stream, a bare component with no calendar around it.
        match calendar.0.first() {
            Some(root) if root.named("VCALENDAR") => {
                reconcile(&mut calendar.0[0], &doc, blocky, &self.types, escaper)
            }
            _ => reconcile(&mut calendar, &doc, blocky, &self.types, escaper),
        }

        Ok(calendar.to_string())
    }

    /// Render the given specs as `[[block]]`s under the `VCALENDAR`.
    fn project_blocks(&self, specs: &[&Spec]) -> String {
        let mut out = String::new();

        out.push_str("# iCalendar as TOML, edited by tCal.\n");
        out.push_str("#\n");
        out.push_str("# Each component is a [[block]]; repeat a block for repeated\n");
        out.push_str("# components, delete one you do not need. Empty fields and empty\n");
        out.push_str("# blocks are ignored. Properties and component types tCal does\n");
        out.push_str("# not model are kept verbatim, not shown here.\n");

        let tops = self.calendar.top_level();

        for spec in specs {
            let instances: Vec<&IcalCst<'_>> = tops
                .iter()
                .copied()
                .filter(|component| component.named(spec.name))
                .collect();

            if instances.is_empty() {
                project_component(&mut out, None, spec, Some(spec.key));
            } else {
                for component in instances {
                    project_component(&mut out, Some(component), spec, Some(spec.key));
                }
            }
        }

        out
    }

    /// Render one component type flattened as the document root.
    ///
    /// Bare keys sit at the top level, sections like `[[attendee]]` under them,
    /// and no header wraps them. The first component of that type fills it, or
    /// an empty example where there is none.
    fn project_flat(&self, spec: &Spec) -> String {
        let component = self
            .calendar
            .top_level()
            .into_iter()
            .find(|component| component.named(spec.name));

        let mut out = String::new();
        out.push_str("# iCalendar ");
        out.push_str(spec.key);
        out.push_str(" as TOML, edited by tCal.\n");
        out.push_str("#\n");
        out.push_str("# Fill what you need; empty fields are ignored. Other\n");
        out.push_str("# components and properties tCal does not model are kept\n");
        out.push_str("# verbatim, not shown here.\n");
        out.push('\n');

        project_component(&mut out, component, spec, None);
        out
    }
}

/// Reconcile `container` against the edited document.
///
/// Flat mode reads the whole document as the selected type's table, a `VEVENT`
/// by default. Block mode reads each selected type's `[[block]]`s, an empty
/// selection meaning every type. An unselected type is left untouched.
fn reconcile<'a, C: TcalContainer<'a>>(
    container: &mut C,
    doc: &DocumentMut,
    blocky: bool,
    filter: &[&Spec],
    escaper: Escaper,
) {
    if !blocky {
        let spec = filter.first().copied().unwrap_or(&VEVENT);
        let count = usize::from(block_has_content(doc.as_table(), spec));
        container.set_child_count(spec.name, count);
        if let Some(component) = container.children_mut(spec.name).next() {
            apply_component(component, doc.as_table(), spec, escaper);
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

        container.set_child_count(spec.name, blocks.len());
        for (component, table) in container.children_mut(spec.name).zip(blocks) {
            apply_component(component, table, spec, escaper);
        }
    }
}

/// Rewrite one component's fields and children from its TOML table.
///
/// Each field is folded onto the lines the component already holds for it, so
/// a parameter the document does not show survives.
fn apply_component<'a>(
    component: &mut IcalCst<'a>,
    table: &dyn TableLike,
    spec: &Spec,
    escaper: Escaper,
) {
    for field in spec.fields {
        let originals = component.lines(field.name);

        component.set_lines(field.name, &field.content_lines(table, &originals), escaper);
    }

    for child in spec.children {
        let blocks: Vec<&dyn TableLike> = table
            .get(child.key)
            .map(tables)
            .unwrap_or_default()
            .into_iter()
            .filter(|nested| block_has_content(*nested, child))
            .collect();

        component.set_child_count(child.name, blocks.len());
        for (kid, kid_table) in component.children_mut(child.name).zip(blocks) {
            apply_component(kid, kid_table, child, escaper);
        }
    }
}

/// The display group of an inline field, driving the blank line separators.
///
/// The bare scalar keys form one group and the dates another, while each
/// structured field is its own, keyed by field key so that two adjacent
/// durations stay separated.
fn field_group(field: &Field) -> (u8, &str) {
    match field.kind {
        Kind::Date => (1, ""),
        Kind::Duration { .. } => (2, field.key),
        Kind::Recur => (3, field.key),
        _ => (0, ""),
    }
}

/// Render one attendee block under `header`, filled or empty.
fn attendee_block(lines: &mut Vec<Line>, header: &str, entry: Option<&IcalLine<'_>>) {
    lines.push(Line {
        lhs: format!("[[{header}]]"),
        hint: None,
    });

    lines.extend(attendee_keys(entry));
}

/// The keys of one attendee block, without its `[[header]]` line.
///
/// This is what a merge writes when two sides contest one attendee, since
/// repeating the header would make a second attendee rather than a duplicate
/// key.
pub(crate) fn attendee_keys(entry: Option<&IcalLine<'_>>) -> Vec<Line> {
    let mut lines = Vec::new();

    let display_name = entry
        .and_then(|line| line.param_value("CN"))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("display-name = {}", toml_str(&display_name)),
        hint: None,
    });

    let value = entry.map(TcalProp::text).unwrap_or_default();
    lines.push(Line {
        lhs: format!("value = {}", toml_str(strip_mailto(&value))),
        hint: Some("email address".to_owned()),
    });

    let role = entry
        .and_then(|line| line.param_value("ROLE"))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("role = {}", toml_str(&role)),
        hint: Some("chair, req-participant, opt-participant, non-participant".to_owned()),
    });

    let status = entry
        .and_then(|line| line.param_value("PARTSTAT"))
        .unwrap_or_default();
    lines.push(Line {
        lhs: format!("status = {}", toml_str(&status)),
        hint: Some("needs-action, accepted, declined, tentative, delegated".to_owned()),
    });

    lines
}

/// The lines of a component writing a field's property.
///
/// They are empty where the component is absent, as an empty block's is.
pub(crate) fn entries_for<'c, 'a>(
    component: Option<&'c IcalCst<'a>>,
    field: &Field,
) -> Vec<&'c IcalLine<'a>> {
    component
        .map(|component| component.props(field.name).collect())
        .unwrap_or_default()
}

/// The child components of `component` matching a child spec's type.
pub(crate) fn child_components<'c, 'a>(
    component: Option<&'c IcalCst<'a>>,
    child: &Spec,
) -> Vec<&'c IcalCst<'a>> {
    component
        .map(|component| component.children(child.name).collect())
        .unwrap_or_default()
}

/// Render a component as a `[[prefix]]` block.
///
/// Its simple fields become one aligned key block, and its attendee fields and
/// child components nested `[[prefix.key]]` blocks, recursively.
fn project_component(
    out: &mut String,
    component: Option<&IcalCst<'_>>,
    spec: &Spec,
    prefix: Option<&str>,
) {
    // NOTE: `None` is the flat top-level event, so it takes no `[[block]]`
    // header and its sections sit at the top level, `[[attendee]]` rather
    // than `[[x.attendee]]`.
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

    // NOTE: one column for the whole component, so every comment aligns at
    // the same level across the field groups and the attendee section alike.
    let column = comment_column(simple.iter().chain(sections.iter().flatten()));

    emit_lines(out, &simple, column);
    for section in &sections {
        out.push('\n');
        emit_lines(out, section, column);
    }

    for child in spec.children {
        let kids = child_components(component, child);
        let child_prefix = section_header(prefix, child.key);

        if kids.is_empty() {
            project_component(out, None, child, Some(&child_prefix));
        } else {
            for kid in kids {
                project_component(out, Some(kid), child, Some(&child_prefix));
            }
        }
    }
}

/// The TOML header for a section or child `key` under an optional `prefix`.
///
/// That is `"key"` at the flat top level, else `"prefix.key"`.
fn section_header(prefix: Option<&str>, key: &str) -> String {
    match prefix {
        Some(prefix) => format!("{prefix}.{key}"),
        None => key.to_owned(),
    }
}

/// Render an attendee field as one `[[header]]` block per entry.
///
/// A field no entry wrote gets a single empty example instead.
fn attendee_section(entries: &[&IcalLine<'_>], header: &str) -> Vec<Line> {
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

/// Whether a TOML block carries any modeled value.
///
/// That is what tells a real component from an empty example placeholder.
fn block_has_content(table: &dyn TableLike, spec: &Spec) -> bool {
    spec.fields
        .iter()
        .any(|field| !field.content_lines(table, &[]).is_empty())
        || spec.children.iter().any(|child| {
            table
                .get(child.key)
                .map(tables)
                .unwrap_or_default()
                .iter()
                .any(|nested| block_has_content(*nested, child))
        })
}

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned, string::String, vec::Vec};

    use crate::{error::TcalResult, template::TcalTemplate};

    /// Project a calendar, every modelled type as a block.
    fn project(source: &str) -> String {
        TcalTemplate::parse(source).unwrap().project()
    }

    /// Project the given component types, one of them flattening at the root.
    fn project_with(source: &str, types: &[String]) -> TcalResult<String> {
        Ok(TcalTemplate::parse(source)?.with_types(types)?.project())
    }

    /// Project one component type, flattened at the document root.
    fn project_one(source: &str, ty: &str) -> TcalResult<String> {
        project_with(source, &[ty.to_owned()])
    }

    /// Fold an edited document back onto a calendar.
    fn apply(source: &str, edited: &str) -> TcalResult<String> {
        TcalTemplate::parse(source)?.apply(edited)
    }

    /// Fold one back for the given component types alone.
    fn apply_with(source: &str, edited: &str, types: &[String]) -> TcalResult<String> {
        TcalTemplate::parse(source)?
            .with_types(types)?
            .apply(edited)
    }

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
        let ical = SAMPLE;
        let toml = project(ical);

        assert!(toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
        assert!(toml.contains("date-start = 2026-06-13T14:00:00"));
        assert!(toml.contains("date-start-tz = \"America/New_York\""));
        assert!(toml.contains("location = \"Room 1\""));
        assert!(toml.contains("[[event.attendee]]"));
        assert!(toml.contains("value = \"jane@example.com\""));
        assert!(toml.contains("display-name = \"Jane Doe\""));
        assert!(toml.contains("[[event.alarm]]"));
        assert!(toml.contains("action = \"DISPLAY\""));
        assert!(toml.contains("trigger.min = 15"));

        assert!(toml.contains("[[todo]]"));
        assert!(toml.contains("[[journal]]"));
        assert!(toml.contains("[[free-busy]]"));
        assert!(toml.contains("[[timezone]]"));
        assert!(!toml.contains("keep me verbatim"));
        assert!(!toml.contains("DTSTAMP"));
    }

    #[test]
    fn blank_project_shows_every_component_type() {
        let toml = project("");

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
        let ical = SAMPLE;
        let toml = project_one(ical, "event").unwrap();

        assert!(!toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
        assert!(toml.contains("[[attendee]]"));
        assert!(toml.contains("[[alarm]]"));

        assert!(project_one(ical, "nope").is_err());
    }

    #[test]
    fn project_one_round_trips_flat() {
        let ical = SAMPLE;
        let toml = project_one(ical, "event").unwrap();

        assert_eq!(apply(SAMPLE, &toml).unwrap(), SAMPLE);
    }

    #[test]
    fn richer_calendar_projects_blocks() {
        let src = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\nSUMMARY:a\r\nEND:VEVENT\r\n\
            BEGIN:VEVENT\r\nSUMMARY:b\r\nEND:VEVENT\r\n\
            END:VCALENDAR\r\n";
        let ical = src;
        let toml = project(ical);

        assert_eq!(toml.lines().filter(|line| *line == "[[event]]").count(), 2);
        assert!(toml.contains("[[event.alarm]]"));
    }

    #[test]
    fn blank_project_layout() {
        let toml = project("");

        assert!(!toml.contains("uid"));
        assert!(toml.find("summary =").unwrap() < toml.find("date-start =").unwrap());
        assert!(toml.find("date-start =").unwrap() < toml.find("date-end =").unwrap());
        assert!(toml.find("description =").unwrap() < toml.find("[[event.attendee]]").unwrap());

        assert!(toml.contains("summary = \"\""));
        assert!(toml.contains("description = \"\""));
        assert!(!toml.contains("#summary"));

        assert!(!toml.contains("# required"));
        assert!(toml.contains("# 2026-06-13T14:30:00"));
        assert!(toml.contains("# display, email, audio"));
        assert!(toml.contains("# confirmed, tentative, cancelled"));
        assert!(!toml.contains("e.g."));

        assert!(toml.contains("recurrence.frequency = \"\""));
        assert!(!toml.contains("[event.recurrence]"));
        assert!(
            toml.find("recurrence.frequency").unwrap() < toml.find("[[event.attendee]]").unwrap()
        );
    }

    #[test]
    fn hints_are_tab_aligned() {
        let toml = project("");

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

    /// The minimal-diff guarantee at its limit: an untouched buffer moves not
    /// one byte.
    #[test]
    fn apply_projection_is_a_no_op() {
        let ical = SAMPLE;
        let toml = project(ical);

        assert_eq!(apply(SAMPLE, &toml).unwrap(), SAMPLE);
    }

    #[test]
    fn apply_changes_only_the_edited_line() {
        let ical = SAMPLE;
        let toml = project(ical).replace("Team sync", "Team lunch");

        let out = apply(SAMPLE, &toml).unwrap();

        assert_eq!(
            out,
            SAMPLE.replace("SUMMARY:Team sync", "SUMMARY:Team lunch")
        );
    }

    #[test]
    fn apply_edits_an_existing_alarm() {
        let toml = project(SAMPLE).replace("trigger.min = 15", "trigger.min = 30");

        let out = apply(SAMPLE, &toml).unwrap();

        assert_eq!(out, SAMPLE.replace("TRIGGER:-PT15M", "TRIGGER:-PT30M"));
    }

    #[test]
    fn apply_roundtrip_preserves_unmodeled() {
        let ical = SAMPLE;
        let toml = project(ical);

        let out = apply(SAMPLE, &toml).unwrap();

        assert!(out.contains("SUMMARY:Team sync"));
        assert!(out.contains("DTSTART;TZID=America/New_York:20260613T140000"));
        assert!(out.contains("mailto:jane@example.com"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
        assert!(out.contains("DTSTAMP:20260101T000000Z"));
        assert!(out.contains("TRIGGER:-PT15M"));
    }

    #[test]
    fn uid_is_hidden_and_app_managed() {
        let ical = SAMPLE;

        let toml = project(ical);
        assert!(!toml.contains("uid"));

        let edited = "[[event]]\nsummary = \"Team sync\"\nuid = \"hacked\"\n";
        let out = apply(SAMPLE, edited).unwrap();
        assert!(out.contains("UID:abc@example"));
        assert!(!out.contains("hacked"));
    }

    #[test]
    fn apply_edits_modeled_field() {
        let edited = "[[event]]\nsummary = \"New title\"\n";

        let out = apply(SAMPLE, edited).unwrap();

        assert!(out.contains("SUMMARY:New title"));
        assert!(!out.contains("Team sync"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_renders_all_day_and_utc_dates() {
        let all_day = apply(SAMPLE, "[[event]]\ndate-start = 2026-12-25\n").unwrap();
        assert!(all_day.contains("DTSTART;VALUE=DATE:20261225"));

        let utc = apply(SAMPLE, "[[event]]\ndate-start = 2026-06-13T14:00:00Z\n").unwrap();
        assert!(utc.contains("DTSTART:20260613T140000Z"));

        let zoned = apply(
            SAMPLE,
            "[[event]]\ndate-start = 2026-06-13T09:30:00\ndate-start-tz = \"Europe/Paris\"\n",
        )
        .unwrap();
        assert!(zoned.contains("DTSTART;TZID=Europe/Paris:20260613T093000"));

        let floating = apply(SAMPLE, "[[event]]\ndate-start = 2026-06-13T09:30:00\n").unwrap();
        assert!(floating.contains("DTSTART:20260613T093000\r\n"));

        let legacy = apply(SAMPLE, "[[event]]\ndate-start = \"2026-06-13 14:00 UTC\"\n").unwrap();
        assert!(legacy.contains("DTSTART:20260613T140000Z"));
    }

    #[test]
    fn apply_adds_an_alarm() {
        let src = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:Solo\r\n\
            END:VEVENT\r\n\
            END:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"Solo\"\n\n\
            [[event.alarm]]\naction = \"DISPLAY\"\ntrigger.min = 10\n";

        let out = apply(src, edited).unwrap();

        assert!(out.contains("BEGIN:VEVENT\r\nSUMMARY:Solo\r\nBEGIN:VALARM\r\n"));
        assert!(out.contains("ACTION:DISPLAY\r\n"));
        assert!(out.contains("TRIGGER:-PT10M\r\n"));
        assert!(out.contains("END:VALARM\r\nEND:VEVENT\r\n"));
    }

    #[test]
    fn apply_removes_an_alarm() {
        let out = apply(SAMPLE, "[[event]]\nsummary = \"Team sync\"\n").unwrap();

        assert!(!out.contains("BEGIN:VALARM"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_empty_buffer_removes_modeled_components() {
        let out = apply(SAMPLE, "").unwrap();

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
        let ical = src;
        let toml = project(ical);

        assert_eq!(toml.matches("[[event]]").count(), 2);

        let edited = toml.replace("second", "2nd");
        let out = apply(src, &edited).unwrap();
        assert_eq!(out, src.replace("SUMMARY:second", "SUMMARY:2nd"));
    }

    #[test]
    fn apply_adds_a_todo() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[todo]]\nsummary = \"Submit report\"\ndate-due = 2026-06-20T17:00:00\n";

        let out = apply(src, edited).unwrap();

        assert!(out.contains("BEGIN:VTODO\r\n"));
        assert!(out.contains("SUMMARY:Submit report\r\n"));
        assert!(out.contains("DUE:20260620T170000\r\n"));
        assert!(out.contains("END:VTODO\r\n"));
    }

    #[test]
    fn apply_uppercases_enum_values() {
        let edited = "[[event]]\nsummary = \"Team sync\"\nstatus = \"confirmed\"\n\n\
            [[event.attendee]]\nvalue = \"jane@example.com\"\n\
            role = \"req-participant\"\nstatus = \"accepted\"\n";

        let out = apply(SAMPLE, edited).unwrap();

        assert!(out.contains("STATUS:CONFIRMED"));
        assert!(out.contains("ROLE=REQ-PARTICIPANT"));
        assert!(out.contains("PARTSTAT=ACCEPTED"));
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
        let toml = project(RECUR_SAMPLE);

        assert!(toml.contains("recurrence.frequency = \"weekly\""));
        assert!(toml.contains("recurrence.interval = 2"));
        assert!(toml.contains("recurrence.by-day = [\"mo\", \"we\", \"fr\"]"));
    }

    #[test]
    fn recurrence_round_trips() {
        let toml = project(RECUR_SAMPLE);

        assert_eq!(apply(RECUR_SAMPLE, &toml).unwrap(), RECUR_SAMPLE);
    }

    #[test]
    fn recurrence_assembles_from_parts() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"x\"\n\n\
            [event.recurrence]\nfrequency = \"monthly\"\nby-month-day = [-1]\n";

        let out = apply(src, edited).unwrap();

        assert!(out.contains("RRULE:FREQ=MONTHLY;BYMONTHDAY=-1\r\n"));
    }

    #[test]
    fn recurrence_until_is_native() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            RRULE:FREQ=DAILY;UNTIL=20261231T235900Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = project(src);

        assert!(toml.contains("recurrence.until = 2026-12-31T23:59:00Z"));
        assert_eq!(apply(src, &toml).unwrap(), src);
    }

    #[test]
    fn recurrence_raw_fallback_for_unmodeled_parts() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            RRULE:FREQ=DAILY;BYHOUR=9\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = project(src);

        assert!(toml.contains("recurrence.rule = \"FREQ=DAILY;BYHOUR=9\""));
        assert_eq!(apply(src, &toml).unwrap(), src);
    }

    const DURATION_SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
        BEGIN:VEVENT\r\n\
        SUMMARY:Workshop\r\n\
        DURATION:P1DT2H30M\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    #[test]
    fn duration_projects_structured_parts() {
        let toml = project(DURATION_SAMPLE);

        assert!(toml.contains("duration.day = 1"));
        assert!(toml.contains("duration.hour = 2"));
        assert!(toml.contains("duration.min = 30"));
        assert!(toml.contains("duration.week = \"\""));
    }

    #[test]
    fn duration_round_trips() {
        let toml = project(DURATION_SAMPLE);

        assert_eq!(apply(DURATION_SAMPLE, &toml).unwrap(), DURATION_SAMPLE);
    }

    #[test]
    fn duration_assembles_with_implied_sign() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let edited = "[[event]]\nsummary = \"x\"\nduration.hour = 1\nduration.min = 30\n\n\
            [[event.alarm]]\naction = \"DISPLAY\"\ntrigger.min = 15\n";

        let out = apply(src, edited).unwrap();

        assert!(out.contains("DURATION:PT1H30M\r\n"));
        assert!(out.contains("TRIGGER:-PT15M\r\n"));
    }

    #[test]
    fn duration_lone_week_stays_weekly() {
        let src = "BEGIN:VCALENDAR\r\nEND:VCALENDAR\r\n";
        let out = apply(src, "[[event]]\nsummary = \"x\"\nduration.week = 2\n").unwrap();

        assert!(out.contains("DURATION:P2W\r\n"));
    }

    #[test]
    fn trigger_raw_fallback_for_date_time() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            BEGIN:VALARM\r\nACTION:DISPLAY\r\n\
            TRIGGER;VALUE=DATE-TIME:20260101T120000Z\r\n\
            END:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = project(src);

        assert!(toml.contains("trigger.raw = "));
        let out = apply(src, &toml).unwrap();
        assert_eq!(out, src);
    }

    /// Real VTIMEZONE exports were losing their offsets, so what the calendar
    /// wrote has to survive a fold-back.
    #[test]
    fn timezone_offsets_round_trip() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nTZID:Europe/Paris\r\n\
            BEGIN:STANDARD\r\nDTSTART:19701025T030000\r\nTZOFFSETFROM:+0200\r\n\
            TZOFFSETTO:+0100\r\nTZNAME:CET\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\nEND:VCALENDAR\r\n";
        let toml = project(src);

        assert!(toml.contains("offset-from = \"+0200\""));
        assert!(toml.contains("offset-to = \"+0100\""));
        assert_eq!(apply(src, &toml).unwrap(), src);
    }

    /// The twin of the time zone offset bug: periods were vanishing rather
    /// than projecting as period strings.
    #[test]
    fn freebusy_periods_round_trip() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VFREEBUSY\r\nUID:fb@x\r\n\
            DTSTART:19980101T000000Z\r\nDTEND:19980101T060000Z\r\n\
            FREEBUSY:19980101T010000Z/19980101T020000Z,19980101T030000Z/PT1H\r\n\
            END:VFREEBUSY\r\nEND:VCALENDAR\r\n";
        let toml = project(src);

        assert!(toml.contains(
            "periods = [\"19980101T010000Z/19980101T020000Z\", \"19980101T030000Z/PT1H\"]"
        ));
        assert_eq!(apply(src, &toml).unwrap(), src);
    }

    #[test]
    fn attendee_display_name_leads() {
        let toml = project(SAMPLE);
        let block = toml.split("[[event.attendee]]").nth(1).unwrap();
        assert!(block.find("display-name =").unwrap() < block.find("value =").unwrap());

        let edited = "[[event]]\nsummary = \"x\"\n\n\
            [[event.attendee]]\ndisplay-name = \"Jane Doe\"\nvalue = \"jane@example.com\"\n";
        let out = apply(SAMPLE, edited).unwrap();
        assert!(out.contains("CN=Jane Doe"));
    }

    #[test]
    fn apply_keeps_the_parameters_the_form_does_not_show() {
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\n\
            SUMMARY;LANGUAGE=en:Team sync\r\n\
            ATTENDEE;RSVP=TRUE;PARTSTAT=ACCEPTED;CUTYPE=INDIVIDUAL:mailto:jane@example.com\r\n\
            END:VEVENT\r\nEND:VCALENDAR\r\n";

        let edited = "[[event]]\nsummary = \"Team lunch\"\n\n\
            [[event.attendee]]\nvalue = \"jane@example.com\"\nstatus = \"\"\n";

        let out = apply(src, edited).unwrap();

        assert!(out.contains("SUMMARY;LANGUAGE=en:Team lunch"));
        assert!(out.contains("ATTENDEE;RSVP=TRUE;CUTYPE=INDIVIDUAL:mailto:jane@example.com"));
    }

    #[test]
    fn alarm_separates_trigger_and_duration() {
        let toml = project("");
        assert!(toml.contains("trigger.sec = \"\"\n\nduration.week = \"\""));
    }

    #[test]
    fn project_with_no_flags_shows_all() {
        let ical = SAMPLE;
        assert_eq!(project_with(ical, &[]).unwrap(), project(ical));
    }

    #[test]
    fn project_with_one_flag_flattens() {
        let ical = SAMPLE;
        let toml = project_with(ical, &["event".to_owned()]).unwrap();

        assert!(!toml.contains("[[event]]"));
        assert!(toml.contains("summary = \"Team sync\""));
    }

    #[test]
    fn project_with_many_flags_filters_blocks() {
        let ical = SAMPLE;
        let toml = project_with(ical, &["event".to_owned(), "todo".to_owned()]).unwrap();

        assert!(toml.contains("[[event]]"));
        assert!(toml.contains("[[todo]]"));
        assert!(!toml.contains("[[journal]]"));
        assert!(!toml.contains("[[timezone]]"));
    }

    #[test]
    fn apply_with_filter_preserves_unselected_block() {
        let edited = "[[todo]]\nsummary = \"Submit report\"\n";
        let out = apply_with(SAMPLE, edited, &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VTODO"));
        assert!(out.contains("SUMMARY:Submit report"));
        assert!(out.contains("SUMMARY:Team sync"));
        assert!(out.contains("X-CUSTOM:keep me verbatim"));
    }

    #[test]
    fn apply_with_filter_does_not_remove_unselected() {
        let out = apply_with(SAMPLE, "[[todo]]\nsummary = \"\"\n", &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VEVENT"));
        assert!(out.contains("SUMMARY:Team sync"));
        assert!(!out.contains("BEGIN:VTODO"));
    }

    #[test]
    fn apply_with_flat_one_type_merges() {
        let ical = SAMPLE;
        let toml = project_with(ical, &["todo".to_owned()]).unwrap();
        let filled = toml.replace("summary = \"\"", "summary = \"My task\"");

        let out = apply_with(SAMPLE, &filled, &["todo".to_owned()]).unwrap();

        assert!(out.contains("BEGIN:VTODO"));
        assert!(out.contains("SUMMARY:My task"));
        assert!(out.contains("SUMMARY:Team sync"));
    }

    #[test]
    fn fields_are_grouped() {
        let toml = project("");
        let at = |needle: &str| toml.find(needle).unwrap();

        assert!(at("summary =") < at("description ="));
        assert!(at("description =") < at("categories ="));
        assert!(at("transparency =") < at("date-start ="));
        assert!(at("date-start =") < at("date-end ="));
        assert!(at("date-end =") < at("duration.week ="));
        assert!(at("duration.sec =") < at("recurrence.frequency ="));
        assert!(at("recurrence.week-start =") < at("[[event.attendee]]"));
    }
}
