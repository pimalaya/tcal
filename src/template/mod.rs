//! Projection between a calcard [`ICalendar`] and an ergonomic TOML buffer.
//!
//! [`project`] renders the calendar as fillable `[[block]]`s; [`project_with`]
//! narrows to chosen types (one flattens at the root, the [`project_one`]
//! view). [`apply`] / [`apply_with`] fold an edited buffer back onto the
//! original text through the format-preserving [`crate::edit`], touching only
//! changed lines and keeping everything unmodeled or unselected byte-for-byte.
//! The modeled vocabulary lives in [`model`]; value conversions in [`datetime`],
//! [`duration`], [`recurrence`]. `UID` and `DTSTAMP` are app-managed, not
//! modeled.

mod datetime;
mod duration;
mod line;
mod model;
mod recurrence;
mod util;

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use calcard::icalendar::{
    ICalendar, ICalendarComponent, ICalendarComponentType, ICalendarEntry, ICalendarParameterName,
};
use toml_edit::{DocumentMut, TableLike};

use crate::{
    edit::tree::{Calendar, Component, Container},
    error::{Result, TcalError},
    template::{
        line::{Line, comment_column, emit_lines},
        model::{Field, Kind, Spec, TOP_LEVEL, VEVENT},
        util::{entry_text, param, strip_mailto, tables, toml_str},
    },
};

/// Project the whole calendar: every modeled type as a `[[block]]`, actual
/// instances filled and one empty example per absent type.
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
    out.push('\n');

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

/// Apply an edited TOML buffer onto the original iCalendar text, re-rendering
/// only changed lines and keeping everything unmodeled byte-for-byte. Filled
/// blocks update or add components; cleared blocks remove them.
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

    let mut cal = Calendar::parse(original_src);

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
fn apply_component(component: &mut Component, table: &dyn TableLike, spec: &Spec) {
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

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned, vec::Vec};

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
        assert!(toml.contains("date-start = 2026-06-13T14:00:00"));
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
        assert!(toml.contains("# 2026-06-13T14:30:00"));
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
        // Native TOML values: a bare date is all-day, a `Z` offset is UTC,
        // and a local date-time with a zone key is a named zone.
        let all_day = super::apply(SAMPLE, "[[event]]\ndate-start = 2026-12-25\n").unwrap();
        assert!(all_day.contains("DTSTART;VALUE=DATE:20261225"));

        let utc = super::apply(SAMPLE, "[[event]]\ndate-start = 2026-06-13T14:00:00Z\n").unwrap();
        assert!(utc.contains("DTSTART:20260613T140000Z"));

        let zoned = super::apply(
            SAMPLE,
            "[[event]]\ndate-start = 2026-06-13T09:30:00\ndate-start-tz = \"Europe/Paris\"\n",
        )
        .unwrap();
        assert!(zoned.contains("DTSTART;TZID=Europe/Paris:20260613T093000"));

        // A floating local date-time with no zone key stays floating.
        let floating =
            super::apply(SAMPLE, "[[event]]\ndate-start = 2026-06-13T09:30:00\n").unwrap();
        assert!(floating.contains("DTSTART:20260613T093000\r\n"));

        // The older friendly string form is still accepted.
        let legacy =
            super::apply(SAMPLE, "[[event]]\ndate-start = \"2026-06-13 14:00 UTC\"\n").unwrap();
        assert!(legacy.contains("DTSTART:20260613T140000Z"));
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
        let edited = "[[todo]]\nsummary = \"Submit report\"\ndate-due = 2026-06-20T17:00:00\n";

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
    fn recurrence_until_is_native() {
        // UNTIL projects as a native TOML date-time and reassembles to digits.
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:x\r\n\
            RRULE:FREQ=DAILY;UNTIL=20261231T235900Z\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains("recurrence.until = 2026-12-31T23:59:00Z"));
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
    fn timezone_offsets_round_trip() {
        // calcard stores TZOFFSETFROM/TO as date-times; they must project as
        // ±HHMM and survive apply, not get dropped (regression: real
        // VTIMEZONE exports were losing their offsets).
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VTIMEZONE\r\nTZID:Europe/Paris\r\n\
            BEGIN:STANDARD\r\nDTSTART:19701025T030000\r\nTZOFFSETFROM:+0200\r\n\
            TZOFFSETTO:+0100\r\nTZNAME:CET\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains("offset-from = \"+0200\""));
        assert!(toml.contains("offset-to = \"+0100\""));
        assert_eq!(super::apply(src, &toml).unwrap(), src);
    }

    #[test]
    fn freebusy_periods_round_trip() {
        // FREEBUSY periods are a Period value type (no borrowed text); they
        // must project as period strings and survive apply, not vanish
        // (regression, twin of the time-zone offset bug).
        let src = "BEGIN:VCALENDAR\r\nBEGIN:VFREEBUSY\r\nUID:fb@x\r\n\
            DTSTART:19980101T000000Z\r\nDTEND:19980101T060000Z\r\n\
            FREEBUSY:19980101T010000Z/19980101T020000Z,19980101T030000Z/PT1H\r\n\
            END:VFREEBUSY\r\nEND:VCALENDAR\r\n";
        let toml = super::project(&ical::parse(src).unwrap());

        assert!(toml.contains(
            "periods = [\"19980101T010000Z/19980101T020000Z\", \"19980101T030000Z/PT1H\"]"
        ));
        assert_eq!(super::apply(src, &toml).unwrap(), src);
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
