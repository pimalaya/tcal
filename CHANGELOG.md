# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Read a merge conflict left before right, following the `IcalMergeConflict` rename in ical-rs.

  The library renamed the field carrying the left side's action from `reason` to `left` and made it the first of the pair, so a conflict reads `{ left, right }` as vcard-rs's twin does. `Sides::read` takes the two in that order. The reading is unchanged: the same conflicts become the same header notes and the same contested keys. This raises the ical-rs requirement to 0.5.

### Added

- Added a `merge` verb, and the `merge` module behind it: a three-way merge projected as a TOML document to decide.

  `tcal merge BASE LOCAL REMOTE OUTPUT` runs the ical-rs three-way merge in process, projects the merged calendar, opens `$EDITOR`, and writes the output path only once the edited document parses. A calendar that does not read refuses the merge naming the side it was given as, so the reader is told which of the three files to open. A property both sides changed is written once per side, each line naming its side, with the ancestor commented above them: TOML forbids duplicate keys, so an undecided document cannot be applied, and the refusal names the property left undecided, under the dotted key the document writes it as, rather than reporting a syntax error. Deciding it is deleting the lines that are not wanted. A value the form writes as several lines (a date and its zone, a duration, a recurrence) contests the lines its two sides spell differently and writes the rest once, so deleting one line of a side still refuses while a line either side would write the same way is never made a choice. What the merge settled on its own becomes a header comment instead of a choice: a removal against an update, a recurrence rule against an overriding instance, and a list both sides edited, whose items are all kept since RFC 5545 gives them no order. A collision on something the projection does not show is a comment too, since there is no key to write twice, and it names the local value as the one kept, as does a collision the form spells the same way on both sides: the local calendar is the merge's left side, which is git's `ours`, so it keeps its value where nothing else settles a collision and the merged bytes are its own. tCard places it the same way. A collision inside an alarm or an attendee decomposes into per-key duplicates within the single table that projects it, never a repeated array-of-tables header, which would silently make a second alarm rather than an error; which attendee is contested is read from the calendar address the merge report carries rather than from a position, which a removal on either side moves. Each header comment is wrapped to the column the rest of the document keeps, a continuation line indented under the text of its bullet. The header announces exactly the contests written below it, and a collision the document finds no line for is said in a comment rather than counted and dropped. The merged calendar is projected as the merge left it rather than written out and read back, so what a reader decides is what the reconciliation produced.

- Offered to re-edit on a broken `edit` buffer instead of discarding it.

  When the edited TOML fails to parse, `edit` now shows the parse error and prompts to re-open `$EDITOR` seeded with the user's own buffer, looping until it parses or the user declines. JSON output stays non-interactive: the error just propagates.

- Projected date and date-time properties as native TOML values instead of quoted strings.

  `date-start`, `date-end`, `date-due`, `date-completed`, the time-zone rule `date-start`, and the recurrence `until` now project as a TOML `date` (all-day), a local `datetime` (floating or zoned), or an offset `datetime` with `Z` (UTC), so editors and tooling can treat them as real dates. A named zone is carried in the adjacent `date-start-tz` key, which now appears only for a named zone, since UTC and floating values fold their zone into the value. `TcalTemplate::apply` reads the native value back and still accepts the older friendly `YYYY-MM-DD HH:MM[ UTC]` string form.

- Replaced the `-C`/`--component` option with cumulative per-type flags (`--event`, `--todo`, `--journal`, `--free-busy`, `--timezone`) on both `template` and `edit`.

  No flag shows the whole calendar (every type as a `[[block]]`, the default). A single flag flattens that type as the document root; two or more keep the `VCALENDAR` root but show only the chosen types. A filtered view only ever reconciles the types it shows, so the unselected ones are kept byte-for-byte on save: editing a `VEVENT` source with `--todo` shows an empty to-do and, once filled, merges it in as a new component beside the untouched event. The library gains `TcalTemplate::with_types`, taking the selected type keys.

- Projected durations (`DURATION`, and an alarm `TRIGGER` offset) as structured dotted `duration.*` keys.

  A duration breaks into `duration.week`/`duration.day`/`duration.hour`/`duration.min`/`duration.sec` magnitude keys, mirroring the recurrence layout. The sign is implied by context rather than typed (a `TRIGGER` fires before the event, so it is negative; a plain `DURATION` is positive), so the parts are always unsigned. On apply the parts reassemble into a canonical iCalendar duration (a lone week stays `P<n>W`, otherwise weeks fold into days). A value that is not a plain duration (an absolute date-time trigger) falls back to a raw `duration.raw = "..."` key and is kept rather than dropped.

- Grouped the form's fields by shape and switched comment alignment to tabs.

  Within each component the fields now cluster by shape, separated by blank lines: the bare scalar keys (`summary`/`description` leading), then the dates, the duration, and the recurrence, each its own group, with the sectioned `attendee` last. Inline `#` comments are padded with tabs instead of spaces and aligned to a single column across the whole block (groups and attendee section alike), padding past the longest line so every comment reliably reaches the column.

- Projected the recurrence rule (`RRULE`) as structured dotted `recurrence.*` keys instead of a raw string.

  The rule's parts become friendly keys (`recurrence.frequency`, `recurrence.interval`, `recurrence.count`, `recurrence.until`, `recurrence.by-day`, `recurrence.by-month`, `recurrence.by-month-day`, `recurrence.by-position`, `recurrence.week-start`): `frequency`/`week-start`/`by-day` read lowercase and uppercase to the canonical form on export, `until` is a friendly date, and the `by-*` parts are arrays of numbers (`by-day` of weekday strings). On apply the parts are reassembled in a canonical token order, so a rule already written that way round-trips byte-for-byte. A rule that uses a part tcal does not model (`BYHOUR`, `RSCALE`, ...) is shown instead as a single raw `recurrence.rule = "..."` key and carried through intact, and that key also works as a manual escape hatch.

- Gave the modeled vocabulary friendlier TOML keys, decoupled from the iCalendar property names.

  Components drop their `V` prefix (`event`, `todo`, `journal`, `free-busy`, `timezone`, with nested `alarm` / `standard` / `daylight`), and cryptic property names become readable (`RRULE` to `recurrence`, `TZID` to `id`, `TZOFFSETFROM`/`TZOFFSETTO` to `offset-from`/`offset-to`, `TZNAME` to `name`, `FREEBUSY` to `periods`, and an attendee's `CN`/`PARTSTAT` to `display-name`/`status`). Date-time keys are prefixed `date-` (`date-start`, `date-end`, `date-due`, `date-completed`), with the time-zone companion `date-start-tz`; their hints show a concrete example date-time. Numeric properties (`priority`, `percent`, `repeat`) render as plain TOML numbers, and `description` is a plain string. Enumerated properties (`status`, `class`, `transparency`, `action`) and the attendee `role`/`status` parameters are listed lowercase in their hints and uppercased to the canonical iCalendar form on export. Field hints drop the `e.g.` prefix in favour of bare variant lists or format strings, the `required` marker is gone (omitting a field just drops it), and the calendar-address hint reads `email address`.

- Added the `TcalTemplate` projection between an iCalendar and an ergonomic TOML buffer.

  A body is read once, by [ical-rs](https://crates.io/crates/ical-rs)'s byte-faithful syntax tree, so a value reaches the document as the calendar wrote it: an escape inside a list item, an absolute alarm trigger, a UTC offset and a `VFREEBUSY` period all keep their bytes rather than being respelled by a reader on the way in.

  `TcalTemplate::project` emits a fillable TOML form rooted at the `VCALENDAR`: every modeled component type (`event`, `todo`, `journal`, `free-busy`, `timezone`) is listed as a `[[block]]` with nested children (`[[event.alarm]]`, `[[event.attendee]]`, `[[timezone.standard]]`/`[[timezone.daylight]]`) hanging off their parent: the actual instances filled in (repeated as needed), plus one empty example for each absent type, so the scaffold doubles as documentation. `TcalTemplate::with_types` narrows that to a chosen set of types (one flattened at the root as bare keys with top-level `[[attendee]]`/`[[alarm]]`, two or more filtered under the `VCALENDAR`), surfaced by the CLI's per-type flags. `TcalTemplate::apply` detects which shape the buffer is (a component-type key means blocks; otherwise a flat single component) and reconciles only the selected types. Fields are uncommented and empty (an empty value is ignored, like a removed line), prefilled when present, and carry an inline `# ...` hint only where the value is not self-evident. Cryptic date-times become a friendly `2026-06-13 14:00` (with all-day, UTC and a broken-out `date-start-tz` time-zone key), and attendees expand into `display-name` / `value` / `role` / `status`. `UID` and `DTSTAMP` are not modeled: they are app-managed (seeded for new events, preserved otherwise) and cannot be set through the buffer. `TcalTemplate::apply` patches the modeled components back onto the byte-faithful syntax tree the projection walked, re-rendering only the lines that actually changed; a filled block updates or adds a component, an empty or absent block removes it, and every unmodeled property (`UID`, `DTSTAMP`, `SEQUENCE`, custom `X-*`), every unmodeled component type, and all folding, casing and ordering are kept byte-for-byte, since the TOML is an editing affordance rather than an interchange format. A modeled property is patched rather than rebuilt, so the parameters the form does not show (`RSVP`, `CUTYPE`, `SENT-BY`, `ALTREP`, `LANGUAGE`, ...) stay in the place they held, while the ones it shows (an attendee's `CN`/`ROLE`/`PARTSTAT`, a date's `TZID`/`VALUE`) are the buffer's to set and to clear.

- Added the `tcal` CLI with two verbs.

  `template [SOURCE]` prints the TOML scaffold (blank or prefilled). `edit [SOURCE]` runs the full "project to `$EDITOR` to apply" round-trip and emits the resulting iCalendar, writing a file source back in place. `SOURCE` resolves deterministically: `-` reads stdin, an existing file is read, otherwise the value is treated as literal iCalendar contents, and omitting it starts from a blank template. New (sourceless) events are seeded with a fresh `urn:uuid` v4 `UID` and a current `DTSTAMP`. `--help` closes on the shared Pimalaya footer, the bug tracker and the sponsoring links, and the version is propagated to every verb.

- Added the crate architecture header on lib.rs and main.rs.

  The library's rustdoc is the architecture of the crate: its layers, the projection reading a body once, the merge writing what it could not settle as duplicate keys, and the module layout. The README stays the public presentation rather than doubling as the crate documentation, and the binary's header covers only its own wiring.

### Changed

- Made the library `no_std` (with `alloc`) and gated the binary behind a `cli` feature.

  With no features it is a `no_std` core: parse an iCalendar and project it to TOML and back (`ical`, `template`, `error`). The `cli` feature (the default) adds the binary, the `template` and `edit` commands and the `$EDITOR` integration, pulling in `std`. Library consumers wanting only the projection no longer pay for clap/anyhow/the editor.

- Split the oversized `template` module by domain and added a golden-fixture test database.

  `template` split its value layer and model into template/{line,util,patch,datetime,duration,recurrence,model}.rs, keeping the projection/apply engine and facade in template.rs. New `tests/data/<name>.ics` + `<name>.<mode>.toml` fixtures (crafted plus real-world exports from ical.js, python-icalendar and libical), checked by tests/fixtures.rs: projection equality always, plus byte-exact round-trip unless a `<name>.lossy` marker says the source is not in the form the projection writes back (reordered `RRULE` tokens, all-day dates without `VALUE=DATE`, attendee parameters tcal does not model, ...). Drop a calendar from a bug report in and generate its expected TOML with `tcal template` to grow the database.
