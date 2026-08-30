#![no_std]
#![cfg_attr(docsrs, feature(doc_cfg))]

//! # tCal
//!
//! Editing an iCalendar as ergonomic TOML: a calendar is projected as a
//! fillable form, a person edits that form, and the edits are folded back onto
//! the calendar's own bytes.
//!
//! This header is the architecture; the behaviour behind it is specified
//! capability by capability in the repository's cairn/spec folder.
//!
//! ## Layers
//!
//! The crate does no I/O of its own and owns no protocol or storage logic, so
//! it has neither coroutines nor a client layer. Its core is a total function
//! over strings: iCalendar text in, TOML text out, and back.
//!
//! That core is always compiled and is `no_std` over `alloc`. [`ical`] reads,
//! [`template`] projects and folds back, [`merge`] reconciles two divergent
//! calendars, and [`error`] names every refusal.
//!
//! The `cli` feature adds the verbs and the binary above them, which is the
//! only place a file, the clock or an editor is reached. A library consumer
//! wanting the projection alone pays for none of it.
//!
//! ## The projection
//!
//! A body is read once. [`ical::parse`] turns a whole stream into ical-rs's
//! byte-faithful syntax tree, and every verb walks that one tree, so no value
//! passes through a second reader that might normalise it on the way in, where
//! no test comparing the document against that reader could see it.
//!
//! [`template::project`] walks the tree against the static component and field
//! tables and writes the form, every modelled component type a `[[block]]`
//! with its children hanging off it. What the tables do not model is not
//! shown.
//!
//! A calendar holds several component types, so the form can be narrowed:
//! [`template::project_with`] takes the types to show, one of them flattening
//! at the document root, and [`template::apply_with`] reconciles only those,
//! which is what keeps a filtered edit from dropping the rest.
//!
//! [`template::apply`] folds an edited form back onto the original text,
//! patching a modelled line rather than rebuilding it: only the value the
//! document moved is written anew, and the rest of the line stays the
//! calendar's own bytes, the parameters the form never showed included.
//!
//! That is why applying needs the original text and not just the document:
//! the TOML is an editing affordance, never an interchange format.
//!
//! ## The modelled vocabulary
//!
//! Static tables in model name every component and property the form shows. A
//! spec is one component type, carrying its fields and its children: the top
//! level lists event, todo, journal, free-busy and timezone, and the children
//! are alarms and the timezone's standard and daylight rules.
//!
//! A field decouples the friendly TOML key from the iCalendar property behind
//! it, so `date-start` can read well without `DTSTART` moving, and carries the
//! inline hint and the kind that drives both directions.
//!
//! A kind is one of nine. Beyond the plain scalar, number and list there are
//! an enum whose variants are lowercase in hints and uppercased on export, a
//! date carrying an adjacent timezone key, a calendar address that strips and
//! restores `mailto:`, a UTC offset, and an attendee section.
//!
//! Recurrence and duration are the two that expand into dotted keys, each
//! with a raw escape hatch for a value tcal cannot break apart. `UID` and
//! `DTSTAMP` are deliberately absent, being app-managed: seeded for a new
//! event and preserved for every other one.
//!
//! ## The merge
//!
//! [`merge::Merge`] reconciles a local and a remote calendar against the base
//! they both came from, then renders the outcome through the same projection,
//! so a merge is read and edited in the form everything else is.
//!
//! The report is used for two things only. Its conflicts are addressed onto
//! projected keys by walking the merged calendar along the merge's component
//! path, `UID` then `RECURRENCE-ID` then position among same-named siblings,
//! down to a spec, a field and then the property itself.
//!
//! That last step goes by the identity the report carries, an attendee's
//! calendar address, rather than by a position either side's own removal
//! moves.
//!
//! Each side's spelling of a contested field is rendered by the same field
//! code the projection uses, so a choice and the document around it are
//! written by one path.
//!
//! The local side is the merge's left side, so the merged calendar is built
//! from its bytes and keeps its value where nothing else settles a collision.
//! tcard and neverest place it the same way, which is also what a merge does
//! everywhere else: the side being merged into is the one that wins.
//!
//! What the merge settled by itself is said in a comment at the head of that
//! document. What it could not settle is written once per side, each line
//! naming its side, which makes the same TOML key appear twice.
//!
//! TOML forbids duplicate keys, so an undecided document does not parse.
//! [`merge::Merged::apply`] catches that refusal and names the property left
//! undecided rather than reporting a syntax error, and nothing is written
//! until a person has deleted the line they do not want.
//!
//! That refusal is the forcing mechanism: the document is TOML rather than a
//! report so that a collision cannot be scrolled past.
//!
//! ## Modules
//!
//! [`ical`] is the reader and the byte-preserving edits a fold-back makes
//! through it, [`template`] the projection engine and its facade, [`merge`]
//! the three-way merge over that facade, and [`error`] the crate-wide error
//! enum.
//!
//! The projection's own layer sits under it, private to the crate: model holds
//! the static vocabulary, patch the content-line grammar a fold-back writes
//! through, datetime, duration and recurrence the values with a shape of their
//! own, and line and util the comment alignment and the TOML rendering.
//!
//! The `cli` feature adds the cli module: the three verbs, how each resolves
//! its source, and the editor round trip. The binary above it is wiring only,
//! and says so in its own header.
//!
//! ## The golden fixture database
//!
//! The tests/data directory is a regression database of real and crafted
//! calendars, checked by tests/fixtures.rs. Each `<name>.<mode>.toml` is the
//! expected projection of `<name>.ics` for that mode, which is either `all`
//! or the `_`-joined type keys the projection was narrowed to.
//!
//! The runner asserts that projection for every fixture, and a byte-exact
//! round trip unless a `<name>.lossy` marker says the source is not already
//! in the form the projection writes back.
//!
//! A real-world export is the most valuable case, so adding one is the
//! fastest way to turn a bug report into a regression test; CONTRIBUTING.md
//! carries the steps.
//!
//! ## Known limitations
//!
//! These are deliberate or pending, and they are what the lossy markers
//! record. A recurrence rule is written back with its tokens in one order, so
//! a rule written in another order round trips canonicalised rather than
//! byte-exact.
//!
//! An all-day date written without its parameter is re-emitted with the
//! parameter RFC 5545 asks for, so `DTSTART:20220101` comes back
//! `DTSTART;VALUE=DATE:20220101`.
//!
//! Only an attendee's common name, role and participation status are
//! modelled, so only those can be edited and the rest are kept on the line as
//! they were. The parameters of a categories or free-busy list are not
//! modelled either, and are likewise kept rather than editable.

extern crate alloc;
#[cfg(feature = "cli")]
extern crate std;

#[cfg(feature = "cli")]
pub mod cli;
pub mod error;
pub mod ical;
pub mod merge;
pub mod template;
