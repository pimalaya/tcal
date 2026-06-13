# tcal architecture

Read the [Pimalaya ARCHITECTURE](https://github.com/pimalaya/.github/blob/master/ARCHITECTURE.md) first: it describes the conventions every Pimalaya repository shares (layering, `no_std`, module and error rules, code style, licensing). This document only covers what is specific to tcal, and assumes you know that shared context.

If a statement here conflicts with the code, the code wins; please flag it.

## Where tcal fits

tcal is a **dual library/CLI** crate (org ARCHITECTURE section 4), but a small and unusual one: it does **no I/O of its own and has no protocol or storage logic**, so it has no coroutines and no `client` layer. It is a pure, total function over strings: iCalendar text in, TOML text out, and back. The two layers are therefore:

1. **`no_std` core** (no features): the projection between an iCalendar and an ergonomic TOML buffer (`ical`, `template`, `edit`, `error`).
2. **CLI** (`cli` feature): the binary and its two verbs, plus the `$EDITOR` integration and `std`.

The "sans-I/O" principle still holds, trivially: the core never touches the filesystem, clock or network. The CLI is the only place that reads files, the clock (for `DTSTAMP`) and `$EDITOR`.

## The two directions

tcal converts between a [calcard](https://crates.io/crates/calcard) `ICalendar` and a TOML buffer in two directions:

- **`project` / `project_with`** (read): turn an `ICalendar` into a fillable, commented TOML scaffold. calcard is the reader; it parses and validates values.
- **`apply` / `apply_with`** (write): fold an edited TOML buffer back onto the **original iCalendar text**.

The central decision is that **calcard is used as a reader only**. Its writer normalises folding, parameter casing and property order, so re-serialising a calendar churns lines nobody touched. Instead `apply` patches the original bytes through `crate::edit`, an in-house format-preserving editor (the `toml_edit` analog for iCalendar): it keeps every content line's original bytes and re-renders only the lines whose modeled value actually changed. Its invariant is `Calendar::parse(s).to_string() == s` for any input; on top of it, projecting then applying an untouched buffer reproduces the source byte-for-byte. Everything tcal does not model (other properties, other component types, `UID`, `DTSTAMP`, `SEQUENCE`, `X-*`, folding, casing, order) is carried through verbatim.

This is why `apply` always needs the original text, not just the edited TOML: the TOML is an editing affordance, not an interchange format.

## The modeled vocabulary

What tcal projects is described by static tables in `template/model.rs`:

- A `Spec { key, name, fields, children }` is one component type. `TOP_LEVEL` lists event, todo, journal, free-busy and timezone; `children` are alarms and the timezone's standard/daylight rules.
- A `Field { key, name, hint, kind }` decouples the friendly TOML `key` (`date-start`) from the iCalendar property `name` (`DTSTART`), so keys can be readable without touching parsing or emission.
- The `Kind` enum drives both directions per field: `Scalar`, `Enum` (variants lowercase in hints, uppercased on export), `Number`, `List`, `Date` (friendly value plus an adjacent `<key>-tz` key), `CalAddress` (`mailto:` stripped/added), `Offset` (`±HHMM`), `Attendee` (a `[[...]]` section of `display-name`/`value`/`role`/`status`), and `Recur` / `Duration` (inline dotted `recurrence.*` / `duration.*` keys, each with a raw escape hatch, `recurrence.rule` / `duration.raw`, for values tcal cannot break apart).

`UID` and `DTSTAMP` are intentionally not modeled: they are app-managed, seeded for new events and preserved otherwise.

## Filtering

`project_with` / `apply_with` take a set of selected type keys:

- none selected projects the whole calendar (every type as a `[[block]]`, absent types shown as one empty example);
- one type flattens at the document root (no wrapper);
- two or more keep the `VCALENDAR` root but show only those types.

The key guarantee: `apply` only reconciles the selected types, so a filtered edit never drops the unselected components. Editing a `VEVENT` with `--todo` adds a to-do beside the untouched event.

## Module layout

```
src/
  lib.rs                 no_std setup, module + feature wiring
  error.rs               TcalError + Result
  ical.rs                calcard parse adapter (text -> ICalendar)
  cli.rs                 [cli] binary: Cli/Command, template & edit verbs
  template.rs            projection/apply engine + facade + unit tests
  template/
    model.rs             Kind, Field, Spec, the static field tables, TOP_LEVEL
    line.rs              Line + tab-aligned comment emission
    util.rs              TOML / escape / calcard-value helpers
    datetime.rs          friendly date-times <-> iCalendar digits, offsets
    duration.rs          DURATION / TRIGGER <-> dotted duration.* keys
    recurrence.rs        RRULE <-> dotted recurrence.* keys
  edit.rs                module root for the format-preserving editor
  edit/
    tree.rs              Calendar/Component/Property/Container + Nodes DOM
    parse.rs             Parser: unfold + build the tree
    render.rs            fold content lines, detect end-of-line
```

`template.rs` holds the public facade (`project`, `project_with`, `project_one`, `apply`, `apply_with`) and the projection/apply orchestration; the submodules hold the model and the per-domain value conversions.

## The golden fixture database

`tests/data/` is a regression database of real and crafted calendars, checked by `tests/fixtures.rs`. Each `<name>.<mode>.toml` is the expected projection of `<name>.ics` for `<mode>` (`all`, or `_`-joined type keys like `event`). The runner asserts `project_with == toml` for every fixture, and a byte-exact round-trip (`apply_with` reproduces the source) unless a `<name>.lossy` marker says the source is not already in calcard's canonical form. Real-world exports are the most valuable cases; adding one is the fastest way to turn a bug report into a test (see [CONTRIBUTING.md](./CONTRIBUTING.md)).

## Known limitations

These are deliberate (or pending), and explain the `.lossy` markers:

- **RRULE canonicalisation**: calcard reorders `RRULE` tokens on read (`FREQ, UNTIL, COUNT, INTERVAL, BYDAY, BYMONTHDAY, BYMONTH, BYSETPOS, WKST`), so a rule in another order round-trips canonicalised, not byte-exact.
- **All-day `VALUE=DATE`**: an all-day date written without the parameter (`DTSTART:20220101`) is re-emitted RFC-correct (`DTSTART;VALUE=DATE:20220101`).
- **Attendee parameters**: only `CN`/`ROLE`/`PARTSTAT` are modeled; others (`RSVP`, `CUTYPE`, ...) are dropped when an attendee line is rewritten.
- **List parameters**: `CATEGORIES` / `FREEBUSY` parameters (such as `FBTYPE`) are not modeled.
