//! # Merge
//!
//! Three-way merge, projected as a TOML document to decide.
//!
//! [`TcalMerge`] runs the ical-rs three-way merge over a base calendar and two
//! calendars derived from it, then projects the merged result through
//! [`crate::template`] so that a person can read it.
//!
//! The local side is the merge's left side, so the merged bytes are its own
//! and a property neither side settled keeps its value, which the document
//! then asks about. That is what tcard and neverest do, and what a merge does
//! everywhere else.
//!
//! What the merge settled by itself becomes a comment in the document header.
//! What it could not settle becomes the same key written once per side, which
//! TOML refuses as a duplicate key.
//!
//! An undecided document therefore does not parse and cannot be applied, and
//! deciding it is deleting the lines that are not wanted. [`TcalMerged::apply`]
//! catches that refusal and names the property left undecided rather than
//! reporting a syntax error.
//!
//! A value written as several lines contests the lines its two sides spell
//! differently and writes the rest once, so the reader is asked about what is
//! in dispute and nothing else.
//!
//! Where the two sides spell every line the same, the difference is in
//! something the projection never shows, and the collision becomes a comment
//! like any other it cannot address.
//!
//! A list both sides edited is said there too. Its items merge as a set, which
//! is right for a value RFC 5545 gives no order to, so there is nothing to
//! choose; but the merged value is one neither side wrote, and a reader who is
//! not told cannot review it.
//!
//! Below it, sides reads a conflict against the four calendars, choice is the
//! contested key that reading yields, and document writes both into the
//! projection.

pub(crate) mod choice;
pub(crate) mod document;
pub(crate) mod sides;

use alloc::{
    borrow::ToOwned,
    string::{String, ToString},
    vec::Vec,
};

use ical::tree::{
    cst::IcalCst,
    merge::{IcalComponentPath, IcalMerge, IcalMergeAction, IcalPropPath},
};
use toml_edit::TomlError;

use crate::{
    error::{TcalError, TcalResult},
    ical::TcalCalendar,
    merge::{
        choice::key_of,
        document::Document,
        sides::{Reading, Sides},
    },
    template::TcalTemplate,
};

/// A three-way merge waiting to run.
pub struct TcalMerge<'a> {
    /// The common ancestor both sides were derived from.
    pub base: &'a str,
    /// The edited side.
    ///
    /// Its bytes are the ones the merged calendar is built from, and its value
    /// is the one a collision keeps.
    pub local: &'a str,
    /// The other side.
    pub remote: &'a str,
}

impl TcalMerge<'_> {
    /// Run the merge and project its result.
    pub fn project(self) -> TcalResult<TcalMerged> {
        let base = read(self.base, "base")?;
        let local = read(self.local, "local")?;
        let remote = read(self.remote, "remote")?;

        let report = IcalMerge {
            base: &base,
            left: &local,
            right: &remote,
        }
        .merge();

        let ical = report.merged.to_string();

        // NOTE: the merged calendar is projected as the merge left it rather
        // than written out and read back, so what the reader decides is what
        // the reconciliation actually produced.
        let sides = Sides {
            merged: TcalCalendar::from(report.merged),
            base: TcalCalendar::from(base),
            local: TcalCalendar::from(local),
            remote: TcalCalendar::from(remote),
        };

        let mut notes = Vec::new();
        let mut choices = Vec::new();

        for conflict in &report.conflicts {
            match sides.read(&conflict.right, &conflict.reason) {
                Reading::Note(note) => notes.push(note),
                Reading::Choice(choice) => choices.push(choice),
            }
        }

        sides.note_unions(&mut notes, &report.left, &report.right);

        let projected = TcalTemplate {
            calendar: sides.merged.clone(),
            types: Vec::new(),
        }
        .project();

        let toml = Document {
            toml: &projected,
            notes,
            choices,
        }
        .decorate();

        Ok(TcalMerged { ical, toml })
    }
}

/// A merged calendar and the document that puts its collisions to a reader.
pub struct TcalMerged {
    /// The merged iCalendar text, the source an edited document folds onto.
    pub ical: String,
    /// The TOML projection of it.
    ///
    /// What the merge settled reads as header comments, what it did not as
    /// duplicate keys.
    pub toml: String,
}

impl TcalMerged {
    /// Fold an edited document back onto the merged calendar.
    ///
    /// A collision left undecided is named by the property it lands on, rather
    /// than as the syntax error the duplicate key it is written as would
    /// otherwise be.
    pub fn apply(&self, edited: &str) -> TcalResult<String> {
        let applied = TcalTemplate::parse(&self.ical)?.apply(edited);

        applied.map_err(|err| match err {
            TcalError::ParseToml(err) => match undecided(edited, &err) {
                Some(key) => TcalError::Undecided(key),
                None => TcalError::ParseToml(err),
            },
            err => err,
        })
    }
}

/// Read one side of a merge as a syntax tree, named by the side it is.
fn read<'a>(text: &'a str, side: &'static str) -> TcalResult<IcalCst<'a>> {
    IcalCst::parse(text).map_err(|err| TcalError::ReadCalendar {
        side,
        message: err.to_string(),
    })
}

/// The list an action edited, for the two actions that edit one.
pub(crate) fn edited_items<'p, 'a>(
    action: &'p IcalMergeAction<'a>,
) -> Option<&'p IcalPropPath<'a>> {
    match action {
        IcalMergeAction::ValueItemAdded { at, .. }
        | IcalMergeAction::ValueItemRemoved { at, .. } => Some(at),
        _ => None,
    }
}

/// Whether an action takes something away.
///
/// That is a collision the merge settles on its own, by keeping the data.
pub(crate) fn is_removal(action: &IcalMergeAction<'_>) -> bool {
    matches!(
        action,
        IcalMergeAction::ComponentRemoved { .. }
            | IcalMergeAction::PropRemoved { .. }
            | IcalMergeAction::ValueItemRemoved { .. }
            | IcalMergeAction::ParamRemoved { .. }
    )
}

/// The component an action lands in.
pub(crate) fn path_of<'p, 'a>(action: &'p IcalMergeAction<'a>) -> &'p IcalComponentPath<'a> {
    match action {
        IcalMergeAction::ComponentAdded { at } | IcalMergeAction::ComponentRemoved { at } => at,
        IcalMergeAction::PropAdded { at, .. }
        | IcalMergeAction::PropRemoved { at, .. }
        | IcalMergeAction::ValueChanged { at, .. }
        | IcalMergeAction::ValueItemAdded { at, .. }
        | IcalMergeAction::ValueItemRemoved { at, .. }
        | IcalMergeAction::ParamAdded { at, .. }
        | IcalMergeAction::ParamRemoved { at, .. }
        | IcalMergeAction::ParamChanged { at, .. } => &at.component,
    }
}

/// The property an action lands on, for the actions that land on one.
pub(crate) fn prop_of<'p, 'a>(action: &'p IcalMergeAction<'a>) -> Option<&'p IcalPropPath<'a>> {
    match action {
        IcalMergeAction::ComponentAdded { .. } | IcalMergeAction::ComponentRemoved { .. } => None,
        IcalMergeAction::PropAdded { at, .. }
        | IcalMergeAction::PropRemoved { at, .. }
        | IcalMergeAction::ValueChanged { at, .. }
        | IcalMergeAction::ValueItemAdded { at, .. }
        | IcalMergeAction::ValueItemRemoved { at, .. }
        | IcalMergeAction::ParamAdded { at, .. }
        | IcalMergeAction::ParamRemoved { at, .. }
        | IcalMergeAction::ParamChanged { at, .. } => Some(at),
    }
}

/// The key a TOML error names as duplicated, undecided rather than mistyped.
///
/// The span of a dotted key covers its last segment alone, so the key is read
/// from the line it sits on: a reader told about `min` would find no such key
/// where the document writes `trigger.min`.
fn undecided(edited: &str, err: &TomlError) -> Option<String> {
    if !err.message().starts_with("duplicate key") {
        return None;
    }

    let span = err.span()?;
    let start = edited.get(..span.start)?.rfind('\n').map_or(0, |at| at + 1);
    let line = edited.get(start..)?.lines().next()?;

    Some(key_of(line).trim_matches('"').to_owned())
}

#[cfg(test)]
mod tests {
    use alloc::{
        format,
        string::{String, ToString},
    };

    use crate::{error::TcalError, merge::TcalMerge};

    /// One organised, attended event, the ancestor every case edits.
    const BASE: &str = "BEGIN:VCALENDAR\r\n\
        VERSION:2.0\r\n\
        PRODID:-//Test//EN\r\n\
        BEGIN:VEVENT\r\n\
        UID:e1@example\r\n\
        DTSTAMP:20260101T000000Z\r\n\
        DTSTART:20260105T090000Z\r\n\
        SUMMARY:Standup\r\n\
        ORGANIZER:mailto:chair@example.com\r\n\
        ATTENDEE;PARTSTAT=NEEDS-ACTION:mailto:ada@example.com\r\n\
        BEGIN:VALARM\r\n\
        ACTION:DISPLAY\r\n\
        TRIGGER:-PT15M\r\n\
        END:VALARM\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    /// A series with one overriding instance, for the recurrence case.
    const SERIES: &str = "BEGIN:VCALENDAR\r\n\
        VERSION:2.0\r\n\
        PRODID:-//Test//EN\r\n\
        BEGIN:VEVENT\r\n\
        UID:e1@example\r\n\
        DTSTAMP:20260101T000000Z\r\n\
        DTSTART:20260105T090000Z\r\n\
        SUMMARY:Standup\r\n\
        RRULE:FREQ=DAILY\r\n\
        END:VEVENT\r\n\
        BEGIN:VEVENT\r\n\
        UID:e1@example\r\n\
        RECURRENCE-ID:20260107T090000Z\r\n\
        DTSTAMP:20260101T000000Z\r\n\
        DTSTART:20260107T100000Z\r\n\
        SUMMARY:Standup\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    /// The base with one line replaced.
    fn edited(base: &str, from: &str, to: &str) -> String {
        assert!(base.contains(from), "the base does not hold {from:?}");
        base.replace(from, to)
    }

    #[test]
    fn a_collision_is_written_once_per_side_and_does_not_apply() {
        let local = edited(BASE, "SUMMARY:Standup", "SUMMARY:Daily standup");
        let remote = edited(BASE, "SUMMARY:Standup", "SUMMARY:Team standup");

        let merged = TcalMerge {
            base: BASE,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        assert!(merged.ical.contains("SUMMARY:Daily standup"));

        assert!(merged.toml.contains("# summary = \"Standup\" # base"));
        assert!(merged.toml.contains("summary = \"Daily standup\" # local"));
        assert!(merged.toml.contains("summary = \"Team standup\" # remote"));

        let err = merged.apply(&merged.toml).unwrap_err();
        assert!(
            matches!(&err, TcalError::Undecided(key) if key == "summary"),
            "unexpected error: {err}"
        );

        let decided = merged
            .toml
            .replace("summary = \"Team standup\" # remote\n", "");
        let out = merged.apply(&decided).unwrap();

        assert!(out.contains("SUMMARY:Daily standup"));
        assert!(!out.contains("Team standup"));
    }

    #[test]
    fn an_unprojectable_collision_keeps_the_local_value() {
        let base = edited(BASE, "SUMMARY:Standup", "SUMMARY:Standup\r\nX-FOO:one");
        let local = edited(&base, "X-FOO:one", "X-FOO:two");
        let remote = edited(&base, "X-FOO:one", "X-FOO:three");

        let merged = TcalMerge {
            base: &base,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        assert!(merged.ical.contains("X-FOO:two"));
        assert!(!merged.ical.contains("X-FOO:three"));
        assert!(
            merged.toml.contains("and the local value was"),
            "no comment"
        );
        assert!(!merged.toml.contains("# conflict"), "offered as a choice");
        assert!(merged.apply(&merged.toml).is_ok());
    }

    #[test]
    fn a_contested_alarm_stays_one_alarm() {
        let local = edited(BASE, "TRIGGER:-PT15M", "TRIGGER:-PT30M");
        let remote = edited(BASE, "TRIGGER:-PT15M", "TRIGGER:-PT45M");

        let merged = TcalMerge {
            base: BASE,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        assert_eq!(merged.toml.matches("[[event.alarm]]").count(), 1);
        assert_eq!(merged.toml.matches("action = \"DISPLAY\"").count(), 1);
        assert!(merged.toml.contains("trigger.min = 30 # local"));
        assert!(merged.toml.contains("trigger.min = 45 # remote"));

        let decided: String = merged
            .toml
            .lines()
            .filter(|line| !line.ends_with("# remote"))
            .map(|line| format!("{line}\n"))
            .collect();
        let out = merged.apply(&decided).unwrap();

        assert!(out.contains("TRIGGER:-PT30M"));
    }

    #[test]
    fn a_rule_against_an_instance_is_a_comment() {
        let local = edited(SERIES, "RRULE:FREQ=DAILY", "RRULE:FREQ=WEEKLY");
        let remote = edited(
            SERIES,
            "DTSTART:20260107T100000Z",
            "DTSTART:20260107T110000Z",
        );

        let merged = TcalMerge {
            base: SERIES,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        assert!(merged.ical.contains("RRULE:FREQ=WEEKLY"));
        assert!(merged.ical.contains("DTSTART:20260107T110000Z"));
        assert!(merged.toml.contains("one is a series"), "no comment");
        assert!(!merged.toml.contains("# conflict"), "offered as a choice");
        assert!(merged.apply(&merged.toml).is_ok());
    }

    #[test]
    fn a_removal_against_an_update_is_a_comment() {
        let local = BASE.replace("LOCATION", "X-NONE").to_string();
        let local = edited(&local, "SUMMARY:Standup\r\n", "");
        let remote = edited(BASE, "SUMMARY:Standup", "SUMMARY:Team standup");

        let merged = TcalMerge {
            base: BASE,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        assert!(merged.ical.contains("SUMMARY:Team standup"));
        assert!(
            merged
                .toml
                .contains("removed on local and updated on remote")
        );
        assert!(!merged.toml.contains("# conflict"), "offered as a choice");
        assert!(merged.apply(&merged.toml).is_ok());
    }

    #[test]
    fn an_unreadable_side_is_named() {
        let merged = TcalMerge {
            base: BASE,
            local: BASE,
            remote: "not an iCalendar at all",
        }
        .project();

        let Err(err) = merged else {
            panic!("the unreadable side was read");
        };

        assert!(
            matches!(&err, TcalError::ReadCalendar { side, .. } if *side == "remote"),
            "{err:?}",
        );
    }
}
