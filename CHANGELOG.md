# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-09-01

### Added

- Added `apply <TEMPLATE> [SOURCE]`, which folds an edited TOML document back onto the calendar it was projected from, with no editor in the middle.

  `template` went one way and only `edit` came back, so a form edited by anything other than tCal's own editor, a script, a filter, a graphical app, could not be folded back at all. `tcal apply form.toml event.ics` closes the round trip, `-` reads the document from stdin, and the type flags are the ones the form was projected with. A document that does not parse, or that leaves a merge collision undecided, is an error here rather than a question. It writes the source file back in place, as `edit` does, `--output` sending the result elsewhere.

- Added `--editor <COMMAND>` to `edit` and `merge`, naming the editor for one run, ahead of `$VISUAL` and `$EDITOR`.

  It is spawned on the path of a temporary TOML file it edits in place, so it must block until the edit is done: use `--editor "code --wait"`, not `--editor code`.

- A buffer that does not fold back is now kept and named when you decline to fix it, and when the editor exits non-zero.

  The error carries the path, which is the recovery: what you typed outlives the run that could not use it. A round trip that folded back still removes the file.

### Changed

- **BREAKING**: the editor is now `$VISUAL`, then `$EDITOR`, and nothing after those, where an unset pair used to fall through to a list of platform defaults.

  That list ended in `xdg-open`, `gnome-open`, `kde-open` and a bare `open`, which are file openers rather than editors: they hand the document to whatever the desktop associates with `.toml` and return before it is closed. tCal then read back a document nobody had touched yet and wrote the calendar out unchanged, which a caller spawning tCal reads as an edit given up on. Neither variable set is now a failure naming both of them and `--editor`.

  The [edit](https://crates.io/crates/edit) dependency is gone with it: what is left is a temporary file, a spawn with the three streams inherited, and a read back. tCard made the same move, and the two stay one design.

### Fixed

- Fixed a list property losing the parameters of its own lines when an item was removed.

  A component carrying two properties of one list name showed their items as one array, and folding that array back counted the items off the front, line by line. Removing one item therefore slid every item behind it onto the line before: a `FREEBUSY;FBTYPE=FREE` period was reported busy, and a `CATEGORIES;LANGUAGE=fr` category came back English. Each item now goes back to the line it came out of.

- Fixed items added to a list property each opening a line of their own, which had made the README's own example untrue.

  `categories = ["pimalaya", "cli"]` wrote two `CATEGORIES` properties rather than the one the README documents. A property holding at most one line has nothing to disambiguate, so the array is that line's items and an added one joins it, parameters and all. Where several lines do exist the added items share one new line between them instead of taking one each.

- Fixed an attendee being offered another component's participation statuses.

  RFC 5545 section 3.2.12 closes `PARTSTAT` per component. A `[[todo.attendee]]` now offers `completed` and `in-process`, and a `[[journal.attendee]]` offers neither those nor `tentative` and `delegated`.

## [0.1.0] - 2026-08-31

### Added

- Added `TcalTemplate`, the projection of an iCalendar as an ergonomic TOML form.

  Components drop their `V` and cryptic property names become readable keys. A blank form lists every modelled property with an empty value, so it doubles as documentation. A date projects as a native TOML value, a recurrence and a duration as dotted parts with a raw escape hatch. `UID` and `DTSTAMP` are app-managed: hidden, seeded for a new event, preserved for every other one.

- Added the fold-back, which rewrites only the lines the form changed.

  A calendar is read once, through [ical-rs](https://crates.io/crates/ical-rs)'s byte-faithful tree, and a modelled line is patched rather than rebuilt, so the parameters the form does not show, the folds, the casing and every unmodelled property survive as the calendar wrote them.

- Added `TcalTemplate::with_types`, which narrows the form to the component types a caller asks for.

  One type flattens at the document root, two or more keep the `VCALENDAR` root. A type the form does not show is one the fold-back does not reconcile, so it comes through untouched.

- Added `TcalMerge`, a three-way merge projected as that same form.

  What both sides changed is written once per side under one key, which TOML refuses as a duplicate: an undecided document cannot be applied, and the refusal names the property rather than reporting a syntax error. What the merge settled on its own is said in the document's header instead.

- Added the `tcal` CLI behind the opt-in `cli` feature: `template`, `edit` and `merge`.

  A source is a file, `-` for stdin, literal iCalendar contents, or nothing for a blank form, and the per-type flags narrow what is shown. `edit` re-opens a buffer that does not fold back, seeded with what you wrote.

- Added a `no_std` core over `alloc`, so a library consumer pays for none of the CLI.

- Added the golden fixture database under tests/data: real and crafted calendars asserting the projection and, where the source is already in the form the projection writes back, a byte-exact round trip.

[unreleased]: https://github.com/pimalaya/tcal/compare/v0.2.0..HEAD
[0.2.0]: https://github.com/pimalaya/tcal/compare/v0.1.0..v0.2.0
[0.1.0]: https://github.com/pimalaya/tcal/compare/root..v0.1.0
