# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

[unreleased]: https://github.com/pimalaya/tcal/compare/v0.1.0..HEAD
[0.1.0]: https://github.com/pimalaya/tcal/compare/root..v0.1.0
