# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added `TcalTemplate`, the projection of an iCalendar as an ergonomic TOML form.

  Components drop their `V` (`event`, `todo`, `journal`, `free-busy`, `timezone`, with nested `alarm`, `standard` and `daylight`) and cryptic property names become readable keys (`RRULE` as `recurrence`, `TZOFFSETFROM` as `offset-from`, `FREEBUSY` as `periods`, an attendee's `CN` and `PARTSTAT` as `display-name` and `status`). A blank form lists every modelled property with an empty value, so it doubles as documentation. A date projects as a native TOML date or date-time, a named zone riding in the adjacent `date-start-tz` key; a recurrence and a duration expand into dotted parts, each with a raw escape hatch for a value tCal cannot break apart. `UID` and `DTSTAMP` are app-managed: hidden, seeded for a new event, preserved for every other one.

- Added the fold-back, which rewrites only the lines the form changed.

  A calendar is read once, through [ical-rs](https://crates.io/crates/ical-rs)'s byte-faithful tree, and a modelled line is patched rather than rebuilt. The parameters the form does not show (`RSVP`, `CUTYPE`, `SENT-BY`, `ALTREP`, `LANGUAGE`), the folds, the casing and the ordering all stay as the calendar wrote them, as does every unmodelled property and component type. The parameters the form does show are the document's to set and to clear.

- Added `TcalTemplate::with_types`, which narrows the form to the component types a caller asks for.

  Nothing selected shows every modelled type as a `[[block]]`, one type flattens at the document root, and two or more keep the `VCALENDAR` root while showing only those. A type the form does not show is a type the fold-back does not reconcile, so it comes through untouched: editing an event with `--todo` adds a to-do and leaves the event alone.

- Added `TcalMerge`, a three-way merge projected as that same form.

  What both sides changed is written once per side under one key, with the ancestor commented above it, which TOML refuses as a duplicate key: an undecided document cannot be applied, and the refusal names the property rather than reporting a syntax error. A value the form writes as several lines is contested whole, so half a side cannot be kept. What the merge settled on its own is said in the header instead, a removal against an update and a rule against an overriding instance among it, and the header announces exactly the contests written below it. Which attendee is contested comes from the calendar address the report carries, not from a position a removal on either side would move.

- Added the `tcal` CLI behind the opt-in `cli` feature: `template`, `edit` and `merge`.

  A source is a file, `-` for stdin, literal iCalendar contents, or nothing for a blank form, and the per-type flags (`--event`, `--todo`, `--journal`, `--free-busy`, `--timezone`) narrow what is shown. `edit` opens `$EDITOR` and offers to re-open a buffer that does not fold back, seeded with what you wrote, so a broken edit is never lost. `merge` writes its output only once the edited document parses.

- Added a `no_std` core over `alloc`, so a library consumer pays for none of the CLI.

- Added the golden fixture database under tests/data: real and crafted calendars asserting the projection and, where the source is already in the form the projection writes back, a byte-exact round trip.

[unreleased]: https://github.com/pimalaya/tcal/compare/root..HEAD
