//! # Merge
//!
//! Three-way merge, projected as a TOML document to decide.
//!
//! [`Merge`] runs the ical-rs three-way merge over a base calendar and two
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
//! deciding it is deleting the lines that are not wanted. [`Merged::apply`]
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
    merge::{IcalComponentPath, IcalMerge, IcalMergeAction, IcalMergeReason, IcalPropPath},
};
use toml_edit::TomlError;

use crate::{
    error::{Result, TcalError},
    ical::{Calendar, named, props},
    template::{
        self, attendee_keys, child_components, entries_for,
        model::{Field, Kind, Spec, TOP_LEVEL},
        top_level,
        util::{strip_mailto, text},
    },
};

/// The column a header comment wraps at, its `# ` prefix included.
const WRAP: usize = 66;

/// A three-way merge waiting to run.
pub struct Merge<'a> {
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

impl Merge<'_> {
    /// Run the merge and project its result.
    pub fn project(self) -> Result<Merged> {
        let base = read(self.base, "base")?;
        let local = read(self.local, "local")?;
        let remote = read(self.remote, "remote")?;

        // NOTE: the local side is the left one, so the merged calendar is
        // built from its bytes and keeps its value where nothing else
        // settles a collision. That is what tcard and neverest do, and what
        // a merge does everywhere else: the side being merged into wins.
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
            merged: Calendar::from(report.merged),
            base: Calendar::from(base),
            local: Calendar::from(local),
            remote: Calendar::from(remote),
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

        let toml = decorate(&template::project(&sides.merged), &notes, &choices);

        Ok(Merged { ical, toml })
    }
}

/// A merged calendar and the document that puts its collisions to a reader.
pub struct Merged {
    /// The merged iCalendar text, the source an edited document folds onto.
    pub ical: String,
    /// The TOML projection of it.
    ///
    /// What the merge settled reads as header comments, what it did not as
    /// duplicate keys.
    pub toml: String,
}

impl Merged {
    /// Fold an edited document back onto the merged calendar.
    ///
    /// A collision left undecided is named by the property it lands on, rather
    /// than as the syntax error the duplicate key it is written as would
    /// otherwise be.
    pub fn apply(&self, edited: &str) -> Result<String> {
        template::apply(&self.ical, edited).map_err(|err| match err {
            TcalError::ParseToml(err) => match undecided(edited, &err) {
                Some(key) => TcalError::Undecided(key),
                None => TcalError::ParseToml(err),
            },
            err => err,
        })
    }
}

/// The three calendars a merge read, plus the one it produced.
///
/// Each is parsed the way the projection reads them.
struct Sides<'a> {
    /// The merged calendar, the one the document projects.
    merged: Calendar<'a>,
    /// The common ancestor, whose value a choice comments above the others.
    base: Calendar<'a>,
    /// The edited side.
    local: Calendar<'a>,
    /// The other side.
    remote: Calendar<'a>,
}

impl Sides<'_> {
    /// What one conflict becomes in the document.
    ///
    /// A choice where both sides wrote a value and only a reader can pick, a
    /// comment where the merge already decided. The conflict names the right
    /// side's action, the remote one, and the reason carries the left side's.
    fn read(&self, remote: &IcalMergeAction<'_>, reason: &IcalMergeReason<'_>) -> Reading {
        match reason {
            IcalMergeReason::Recurrence(local) => Reading::Note(format!(
                "{} changed on local and {} on remote: one is a series and the other one of its instances, and both were kept, so the rule may have moved the ground the instance stood on.",
                self.name(local),
                self.name(remote)
            )),

            IcalMergeReason::Divergent(local) if is_removal(local) || is_removal(remote) => {
                Reading::Note(self.dropped(local, remote))
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
    fn note_unions(
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

            if !notes.contains(&note) {
                notes.push(note);
            }
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
enum Reading {
    /// A comment in the document header, for what the merge settled.
    Note(String),
    /// A contested key, for what only a reader can settle.
    Choice(Choice),
}

/// One contested key.
///
/// It carries where it sits in the document, and how each side spells it.
struct Choice {
    /// The block the contested lines sit in, one step per array of tables.
    ///
    /// Each step is the TOML key and the index of the block among its
    /// siblings.
    at: Vec<(&'static str, usize)>,
    /// The field key whose lines are contested.
    ///
    /// It is every key of the block for an attendee, which the projection
    /// writes as a table of its own.
    key: Option<&'static str>,
    /// The comment this becomes where the document holds no line to contest.
    ///
    /// A collision with nowhere to go is still said.
    kept: String,
    /// The ancestor's lines, commented above the choice.
    base: Vec<String>,
    /// The local side's lines.
    local: Vec<String>,
    /// The remote side's lines.
    remote: Vec<String>,
}

impl Choice {
    /// Whether a projected line writes this choice's contested key.
    fn contests(&self, line: &str) -> bool {
        let Some((key, _)) = line.split_once('=') else {
            return false;
        };

        let key = key.trim();

        match self.key {
            None => !key.is_empty(),
            Some(field) => {
                key == field
                    || key
                        .strip_prefix(field)
                        .is_some_and(|rest| rest.starts_with('.') || rest == "-tz")
            }
        }
    }

    /// Every key the two sides write between them.
    ///
    /// They come in the order the local side writes them, a key only the
    /// remote side has last.
    fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = Vec::new();

        for line in self.local.iter().chain(&self.remote) {
            let key = key_of(line);

            if !keys.contains(&key) {
                keys.push(key);
            }
        }

        keys
    }

    /// The keys the two sides spell differently.
    ///
    /// Those are the ones only a reader can settle.
    fn contested(&self) -> Vec<&str> {
        self.keys()
            .into_iter()
            .filter(|key| line_for(&self.local, key) != line_for(&self.remote, key))
            .collect()
    }

    /// Write the contest over the lines it holds.
    ///
    /// A key the sides differ on becomes the ancestor as a comment then one
    /// live line per side, each naming the side it came from. A key they
    /// agree on is written once, either copy being the other.
    fn render(&self, out: &mut String) {
        let contested = self.contested();

        out.push_str(if contested.len() < 2 {
            "# conflict, keep one line\n"
        } else {
            "# conflict, keep one side\n"
        });

        for key in self.keys() {
            if !contested.contains(&key) {
                if let Some(line) = line_for(&self.local, key) {
                    out.push_str(&format!("{line}\n"));
                }

                continue;
            }

            if let Some(line) = line_for(&self.base, key) {
                out.push_str(&format!("# {line} # base\n"));
            }

            if let Some(line) = line_for(&self.local, key) {
                out.push_str(&format!("{line} # local\n"));
            }

            if let Some(line) = line_for(&self.remote, key) {
                out.push_str(&format!("{line} # remote\n"));
            }
        }
    }
}

/// The key a projected line writes, which is its text up to the `=`.
fn key_of(line: &str) -> &str {
    line.split_once('=').map_or(line, |(key, _)| key).trim()
}

/// The line one side writes for a key, absent where that side writes none.
fn line_for<'l>(lines: &'l [String], key: &str) -> Option<&'l str> {
    lines
        .iter()
        .find(|line| key_of(line) == key)
        .map(String::as_str)
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

/// Read one side of a merge as a syntax tree, named by the side it is.
fn read<'a>(text: &'a str, side: &'static str) -> Result<IcalCst<'a>> {
    IcalCst::parse(text).map_err(|err| TcalError::ReadCalendar {
        side,
        message: err.to_string(),
    })
}

/// Write the choices over the lines they contest, then the notes above them.
///
/// The body is written first so the header can announce the contests the
/// document holds rather than the ones the merge reported: a choice the body
/// found no line for falls back to the note it would have been.
fn decorate(toml: &str, notes: &[String], choices: &[Choice]) -> String {
    let mut header = String::new();
    let mut body = String::new();
    let mut here: Vec<(&str, usize)> = Vec::new();
    let mut counts: Vec<(&str, usize)> = Vec::new();
    let mut written = vec![false; choices.len()];

    for line in toml.lines() {
        // NOTE: the projection's own header is every comment line the
        // document opens with, and the notes go under it.
        if body.is_empty() && line.starts_with('#') {
            header.push_str(line);
            header.push('\n');
            continue;
        }

        if let Some(block) = block_header(line) {
            open(&mut here, &mut counts, block);
            body.push_str(line);
            body.push('\n');
            continue;
        }

        let contested = choices
            .iter()
            .position(|choice| addresses(&choice.at, &here) && choice.contests(line));

        match contested {
            Some(at) if !written[at] => {
                choices[at].render(&mut body);
                written[at] = true;
            }
            // NOTE: a choice writes every side's lines at once, so the lines
            // it replaces are dropped rather than written again.
            Some(_) => {}
            None => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }

    let mut said: Vec<&str> = notes.iter().map(String::as_str).collect();

    said.extend(
        choices
            .iter()
            .zip(&written)
            .filter(|(_, written)| !**written)
            .map(|(choice, _)| choice.kept.as_str()),
    );

    let mut out = header;

    preamble(&mut out, &said, written.iter().filter(|w| **w).count());
    out.push_str(&body);

    out
}

/// Write what the reader is asked, then what the merge settled without asking.
///
/// Both go as comments under the projection's own header.
fn preamble(out: &mut String, notes: &[&str], contests: usize) {
    if contests == 0 && notes.is_empty() {
        return;
    }

    out.push_str("#\n");

    if contests > 0 {
        let plural = if contests == 1 { "" } else { "s" };

        comment(
            out,
            &format!(
                "{contests} conflict{plural} below {} yours to decide, written as the same key once per side. Keep one line of each and delete the others, or replace them with a value of your own: TOML forbids duplicate keys, so this document cannot be applied until every one is decided.",
                if contests == 1 { "is" } else { "are" }
            ),
        );
    }

    if !notes.is_empty() {
        if contests > 0 {
            out.push_str("#\n");
        }

        comment(out, "The merge settled these on its own:");
        out.push_str("#\n");

        for note in notes {
            comment(out, &format!("- {note}"));
        }
    }

    out.push_str("#\n");
}

/// Write one comment paragraph, wrapped at [`WRAP`].
///
/// A continuation line of a bullet is indented under its text.
fn comment(out: &mut String, text: &str) {
    let indent = if text.starts_with("- ") { "  " } else { "" };
    let mut line = String::new();

    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + word.len() + 1 > WRAP {
            out.push_str(&format!("# {line}\n"));
            line = indent.to_owned();
        }

        if !line.is_empty() && !line.ends_with(' ') {
            line.push(' ');
        }

        line.push_str(word);
    }

    if !line.trim().is_empty() {
        out.push_str(&format!("# {line}\n"));
    }
}

/// The dotted key of an array of tables header, for a line that is one.
fn block_header(line: &str) -> Option<&str> {
    line.strip_prefix("[[")?.strip_suffix("]]")
}

/// Open a block at the depth its header names.
///
/// The last step of that header becomes the current block there, and every
/// counter nested under it starts again, since those blocks belong to the one
/// just opened.
fn open<'t>(here: &mut Vec<(&'t str, usize)>, counts: &mut Vec<(&'t str, usize)>, header: &'t str) {
    let depth = header.split('.').count();

    let index = match counts.iter_mut().find(|(held, _)| *held == header) {
        Some((_, count)) => {
            *count += 1;
            *count
        }
        None => {
            counts.push((header, 0));
            0
        }
    };

    counts.retain(|(held, _)| {
        !held
            .strip_prefix(header)
            .is_some_and(|rest| rest.starts_with('.'))
    });

    here.truncate(depth - 1);
    here.push((header.rsplit('.').next().unwrap_or(header), index));
}

/// Whether a choice's address is the block currently open.
fn addresses(at: &[(&str, usize)], here: &[(&str, usize)]) -> bool {
    at.len() == here.len() && at.iter().zip(here).all(|(at, here)| at == here)
}

/// Find the component a merge path names in one calendar.
///
/// The address of the block that projects it comes with it.
fn locate<'i, 'a>(ical: &'i Calendar<'a>, path: &IcalComponentPath<'_>) -> Option<Located<'i, 'a>> {
    let root = ical.read()?;
    let mut steps = path.0.iter();

    // NOTE: a bare component stream has no VCALENDAR to address from, so its
    // lone component is the first block and a path addresses what nests in it.
    let mut found = if named(root, "VCALENDAR") {
        let step = steps.next()?;
        let spec = spec_of(TOP_LEVEL, &step.name)?;
        let siblings: Vec<&IcalCst<'a>> = top_level(ical)
            .into_iter()
            .filter(|component| named(component, spec.name))
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
    props(component, name).next().map(text)
}

/// How one side spells a field of the component a path names.
///
/// Its empty value stands where the side carries neither the property nor the
/// component.
fn field_lines(ical: &Calendar<'_>, at: &IcalPropPath<'_>, field: &Field) -> Vec<String> {
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
fn attendee_lines(ical: &Calendar<'_>, at: &IcalPropPath<'_>, field: &Field) -> Vec<String> {
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
        .position(|line| strip_mailto(&text(line)).eq_ignore_ascii_case(strip_mailto(identity)))
}

/// The list an action edited, for the two actions that edit one.
fn edited_items<'p, 'a>(action: &'p IcalMergeAction<'a>) -> Option<&'p IcalPropPath<'a>> {
    match action {
        IcalMergeAction::ValueItemAdded { at, .. }
        | IcalMergeAction::ValueItemRemoved { at, .. } => Some(at),
        _ => None,
    }
}

/// Whether an action takes something away.
///
/// That is a collision the merge settles on its own, by keeping the data.
fn is_removal(action: &IcalMergeAction<'_>) -> bool {
    matches!(
        action,
        IcalMergeAction::ComponentRemoved { .. }
            | IcalMergeAction::PropRemoved { .. }
            | IcalMergeAction::ValueItemRemoved { .. }
            | IcalMergeAction::ParamRemoved { .. }
    )
}

/// The component an action lands in.
fn path_of<'p, 'a>(action: &'p IcalMergeAction<'a>) -> &'p IcalComponentPath<'a> {
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
fn prop_of<'p, 'a>(action: &'p IcalMergeAction<'a>) -> Option<&'p IcalPropPath<'a>> {
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

    use crate::{error::TcalError, merge::Merge};

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

        let merged = Merge {
            base: BASE,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        // NOTE: the merged bytes carry the local value, the left side, though
        // the document still asks rather than keeping it quietly.
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

        let merged = Merge {
            base: &base,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        // NOTE: the projection does not model X-FOO, so there is no key to
        // write twice under and the document says which value it carries.
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

        let merged = Merge {
            base: BASE,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        // NOTE: a repeated [[event.alarm]] header would be a second alarm, so
        // the contest goes inside the one table the projection wrote.
        assert_eq!(merged.toml.matches("[[event.alarm]]").count(), 1);
        assert_eq!(merged.toml.matches("action = \"DISPLAY\"").count(), 1);
        assert!(merged.toml.contains("trigger.min = 30 # local"));
        assert!(merged.toml.contains("trigger.min = 45 # remote"));

        // NOTE: only the part the two sides spell differently is contested, so
        // dropping the remote lines still leaves a whole alarm.
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

        let merged = Merge {
            base: SERIES,
            local: &local,
            remote: &remote,
        }
        .project()
        .unwrap();

        // NOTE: both changes survive, so there is nothing to choose between
        // and the document parses as it stands.
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

        let merged = Merge {
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
        let merged = Merge {
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
