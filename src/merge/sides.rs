//! # Sides
//!
//! The three calendars a merge read and the one it produced, and what each
//! conflict becomes when read against them.
//!
//! A conflict is a choice where both sides wrote a value the projection has a
//! key for and only a reader can pick, and a note where the merge already
//! decided or the projection shows nothing to contest.

use alloc::{
    borrow::ToOwned,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use ical::tree::{
    cst::IcalCst,
    line::IcalLine,
    merge::{IcalComponentPath, IcalMergeAction, IcalMergeReason, IcalPropPath},
};

use crate::{
    ical::{TcalCalendar, TcalComponent, TcalProp},
    merge::{choice::Choice, edited_items, is_removal, path_of, prop_of, push_note},
    template::{
        attendee_keys, child_components, entries_for,
        model::{Field, Kind, Spec, TOP_LEVEL},
        patch::strip_mailto,
    },
};

/// The three calendars a merge read, plus the one it produced.
///
/// Each is parsed the way the projection reads them.
pub struct Sides<'a> {
    /// The merged calendar, the one the document projects.
    pub merged: TcalCalendar<'a>,
    /// The common ancestor, whose value a choice comments above the others.
    pub base: TcalCalendar<'a>,
    /// The edited side.
    pub local: TcalCalendar<'a>,
    /// The other side.
    pub remote: TcalCalendar<'a>,
}

impl Sides<'_> {
    /// What one conflict becomes in the document.
    ///
    /// A choice where both sides wrote a value and only a reader can pick, a
    /// comment where the merge already decided. A conflict names its two sides
    /// left and right, which here are the local and the remote calendar, and
    /// the left one carries with it why the right one did not simply apply.
    pub fn read(&self, left: &IcalMergeReason<'_>, right: &IcalMergeAction<'_>) -> Reading {
        match left {
            IcalMergeReason::Recurrence(local) => Reading::Note(format!(
                "{} changed on local and {} on remote: one is a series and the other one of its instances, and both were kept, so the rule may have moved the ground the instance stood on.",
                self.name(local),
                self.name(right)
            )),

            IcalMergeReason::Divergent(local) if is_removal(local) || is_removal(right) => {
                Reading::Note(self.dropped(local, right))
            }

            IcalMergeReason::Divergent(local) => match self.choice(local) {
                Some(choice) => Reading::Choice(choice),
                None => Reading::Note(self.kept(local)),
            },
        }
    }

    /// The comment for a collision the document cannot put to a reader.
    ///
    /// The merge settled it by keeping the local side's value.
    fn kept(&self, local: &IcalMergeAction<'_>) -> String {
        format!(
            "{}: changed on both sides, and the local value was kept.",
            self.name(local)
        )
    }

    /// The comment for a collision where at least one side took something away.
    ///
    /// The merge settles that on its own, by keeping the data.
    fn dropped(&self, local: &IcalMergeAction<'_>, remote: &IcalMergeAction<'_>) -> String {
        let name = self.name(local);

        let (gone, kept) = if is_removal(local) {
            ("local", "remote")
        } else {
            ("remote", "local")
        };

        if is_removal(local) && is_removal(remote) {
            return format!("{name}: removed on both sides.");
        }

        // NOTE: a property comes back when the other side updated it, but a
        // whole component cannot, having nothing left to come back into, so
        // the comment says which of the two happened here.
        if matches!(
            (local, remote),
            (IcalMergeAction::ComponentRemoved { .. }, _)
                | (_, IcalMergeAction::ComponentRemoved { .. })
        ) {
            let held = locate(&self.merged, path_of(local)).is_some();
            let outcome = if held { "was kept" } else { "is gone" };

            return format!("{name}: removed on {gone} and changed on {kept}, and it {outcome}.");
        }

        format!("{name}: removed on {gone} and updated on {kept}, and the update was kept.")
    }

    /// Say in the header every list both sides edited.
    ///
    /// Merging items as a set is right for a value RFC 5545 gives no order to,
    /// so the merge keeps both and reports no conflict. Saying nothing is what
    /// would be wrong: the merged value is one neither side wrote.
    pub fn note_unions(
        &self,
        notes: &mut Vec<String>,
        local: &[IcalMergeAction<'_>],
        remote: &[IcalMergeAction<'_>],
    ) {
        for action in local {
            let Some(at) = edited_items(action) else {
                continue;
            };

            let both = remote
                .iter()
                .filter_map(edited_items)
                .any(|other| other == at);

            if !both {
                continue;
            }

            let note = format!(
                "{}: both sides changed its list; the items of both were kept.",
                self.name(action)
            );

            push_note(notes, note);
        }
    }

    /// The choice a collision offers.
    ///
    /// There is one only where the collision lands on a property the
    /// projection writes as a key of its own.
    fn choice(&self, local: &IcalMergeAction<'_>) -> Option<Choice> {
        let at = prop_of(local)?;
        let found = locate(&self.merged, &at.component)?;
        let field = field_of(found.spec, &at.name)?;

        // NOTE: an attendee projects as a table rather than a key, and
        // repeating its header would make a second attendee instead of a
        // duplicate key, so the contest goes inside the one table it wrote.
        let choice = if matches!(field.kind, Kind::Attendee) {
            let entries = entries_for(Some(found.component), field);
            let mut address = found.address;
            address.push((field.key, index_of(&entries, at)?));

            Choice {
                at: address,
                key: None,
                kept: self.kept(local),
                base: attendee_lines(&self.base, at, field),
                local: attendee_lines(&self.local, at, field),
                remote: attendee_lines(&self.remote, at, field),
            }
        } else {
            Choice {
                at: found.address,
                key: Some(field.key),
                kept: self.kept(local),
                base: field_lines(&self.base, at, field),
                local: field_lines(&self.local, at, field),
                remote: field_lines(&self.remote, at, field),
            }
        };

        // NOTE: two sides the projection spells the same way differ in
        // something it never shows, so there is nothing to put to a reader.
        if choice.contested().is_empty() {
            return None;
        }

        Some(choice)
    }

    /// How a comment names what an action landed on.
    ///
    /// The block the projection writes it in and the key it writes it as,
    /// falling back to the iCalendar names where the projection models
    /// neither.
    fn name(&self, action: &IcalMergeAction<'_>) -> String {
        let found = locate(&self.merged, path_of(action));

        let place = match &found {
            Some(found) => found
                .address
                .iter()
                .map(|(key, index)| format!("{key} {}", index + 1))
                .collect::<Vec<_>>()
                .join(" / "),
            None => path_of(action)
                .0
                .iter()
                .map(|step| format!("{} {}", step.name, step.key))
                .collect::<Vec<_>>()
                .join(" / "),
        };

        let Some(at) = prop_of(action) else {
            return place;
        };

        let key = found
            .and_then(|found| field_of(found.spec, &at.name))
            .map(|field| field.key.to_owned())
            .unwrap_or_else(|| at.name.to_string());

        format!("{place} / {key}")
    }
}

/// What one conflict becomes in the projected document.
pub enum Reading {
    /// A comment in the document header, for what the merge settled.
    Note(String),
    /// A contested key, for what only a reader can settle.
    Choice(Choice),
}

/// One component found in one calendar.
///
/// That is the component itself, the spec that projects it, and the address
/// of the block it projects to.
struct Located<'i, 'a> {
    /// The component.
    component: &'i IcalCst<'a>,
    /// The spec that projects it.
    spec: &'static Spec,
    /// The address of its block, one step per array of tables.
    address: Vec<(&'static str, usize)>,
}

/// Find the component a merge path names in one calendar.
///
/// The address of the block that projects it comes with it.
fn locate<'i, 'a>(
    ical: &'i TcalCalendar<'a>,
    path: &IcalComponentPath<'_>,
) -> Option<Located<'i, 'a>> {
    let root = ical.read()?;
    let mut steps = path.0.iter();

    // NOTE: a bare component stream has no VCALENDAR to address from, so its
    // lone component is the first block and a path addresses what nests in it.
    let mut found = if root.named("VCALENDAR") {
        let step = steps.next()?;
        let spec = spec_of(TOP_LEVEL, &step.name)?;
        let siblings: Vec<&IcalCst<'a>> = ical
            .top_level()
            .into_iter()
            .filter(|component| component.named(spec.name))
            .collect();
        let (index, component) = pick(&siblings, &step.key)?;

        Located {
            component,
            spec,
            address: vec![(spec.key, index)],
        }
    } else {
        let spec = spec_of(TOP_LEVEL, &root.begin.as_ref()?.raw_value_str())?;

        Located {
            component: root,
            spec,
            address: vec![(spec.key, 0)],
        }
    };

    for step in steps {
        let spec = spec_of(found.spec.children, &step.name)?;
        let siblings = child_components(Some(found.component), spec);
        let (index, component) = pick(&siblings, &step.key)?;

        found.address.push((spec.key, index));
        found.component = component;
        found.spec = spec;
    }

    Some(found)
}

/// The spec projecting a component name, among the ones reachable there.
fn spec_of(specs: &[&'static Spec], name: &str) -> Option<&'static Spec> {
    specs
        .iter()
        .copied()
        .find(|spec| spec.name.eq_ignore_ascii_case(name))
}

/// The field projecting a property name, among a spec's own.
fn field_of(spec: &'static Spec, name: &str) -> Option<&'static Field> {
    spec.fields
        .iter()
        .find(|field| field.name.eq_ignore_ascii_case(name))
}

/// The sibling a merge step names.
///
/// The one whose `UID` matches, the one whose `UID` matches and overrides an
/// instance where the step does, or the position the step carries for a
/// component with no `UID` at all.
fn pick<'i, 'a>(siblings: &[&'i IcalCst<'a>], key: &str) -> Option<(usize, &'i IcalCst<'a>)> {
    let (uid, instance) = match key.split_once('/') {
        Some((uid, _)) => (uid, true),
        None => (key, false),
    };

    let found = siblings
        .iter()
        .position(|component| {
            text_of(component, "UID").as_deref() == Some(uid)
                && text_of(component, "RECURRENCE-ID").is_some() == instance
        })
        .or_else(|| key.parse().ok().filter(|index| *index < siblings.len()))?;

    Some((found, siblings[found]))
}

/// The text of a component's first property of this name.
fn text_of(component: &IcalCst<'_>, name: &str) -> Option<String> {
    component.props(name).next().map(TcalProp::text)
}

/// How one side spells a field of the component a path names.
///
/// Its empty value stands where the side carries neither the property nor the
/// component.
fn field_lines(ical: &TcalCalendar<'_>, at: &IcalPropPath<'_>, field: &Field) -> Vec<String> {
    let component = locate(ical, &at.component).map(|found| found.component);

    field
        .lines(&entries_for(component, field))
        .into_iter()
        .map(|line| line.lhs)
        .collect()
}

/// The same for an attendee.
///
/// An attendee is one table among the ones its field wrote rather than a key.
fn attendee_lines(ical: &TcalCalendar<'_>, at: &IcalPropPath<'_>, field: &Field) -> Vec<String> {
    let component = locate(ical, &at.component).map(|found| found.component);
    let entries = entries_for(component, field);
    let entry = index_of(&entries, at).map(|index| entries[index]);

    attendee_keys(entry)
        .into_iter()
        .map(|line| line.lhs)
        .collect()
}

/// Which of the entries a field wrote in one calendar a property path names.
///
/// A position is counted in the side the action was read from, so a side's own
/// removal moves it away from what it named elsewhere. An identity, the
/// calendar address of an attendee, names the same property in every calendar.
fn index_of(entries: &[&IcalLine<'_>], at: &IcalPropPath<'_>) -> Option<usize> {
    let Some(identity) = at.identity.as_deref() else {
        return (at.index < entries.len()).then_some(at.index);
    };

    entries
        .iter()
        .position(|line| strip_mailto(&line.text()).eq_ignore_ascii_case(strip_mailto(identity)))
}
