# Contributing guide

Thank you for investing your time in contributing to tcal.

This guide doubles as a deep description of the project: it is written for human contributors *and* for AI assistants, so either can understand the architecture and conventions before changing anything. Read the [Architecture](#architecture) and [Conventions](#conventions) sections before sending a patch.

## Development environment

The environment is managed by [Nix](https://nixos.org/download.html). `nix develop` spawns a shell with the right toolchain; every cargo invocation below assumes it (or run them directly as `nix develop --command cargo ...`).

Without Nix, install a recent stable toolchain via [rustup](https://rust-lang.github.io/rustup/) (`rustup update`); the crate needs Rust matching the `rust-version` in [Cargo.toml](./Cargo.toml).

## Build and feature layers

tcal is a `#![no_std]` library (plus `alloc`) with an optional CLI on top. There is exactly one feature:

- **no features** (`cargo build`): the `no_std` core that fully deals with TOML and iCalendar in both directions (`ical`, `template`, `edit`, `error`). No `std`, no clap, no editor.
- **`cli`** (`cargo build --features cli`): pulls in `std` and the binary, with the `template` and `edit` commands and the `$EDITOR` integration. `cli` is *not* a default feature.

Three configurations are expected to stay green; check them when touching feature gates or imports:

```sh
cargo build                      # no_std core
cargo build --features cli       # full CLI
cargo build --release --features cli
```

## Lint, test, audit

```sh
cargo test                       # unit + integration tests (no_std core)
cargo test --features cli        # also exercises the CLI-only code paths
cargo clippy --all-targets       # and again with --no-default-features
cargo fmt                        # rustfmt; CI checks `cargo fmt --check`
```

Tests come in three kinds:

- **Unit tests** (`#[cfg(test)] mod tests`) in `template.rs` and `edit/tree.rs`, pinning the projection, apply and minimal-diff guarantees on crafted inputs. Because the crate is `no_std`, each test module imports what it needs from `alloc`.
- **Golden fixtures** in `tests/fixtures.rs` over `tests/data/`: each `<name>.<mode>.toml` is the expected projection of `<name>.ics` for `<mode>` (`all`, or `_`-joined type keys like `event` or `event_todo`). The runner asserts `project_with == toml` for every fixture, plus a byte-exact round-trip (`apply_with` reproduces the source) unless a `<name>.lossy` marker says the source is not in calcard's canonical form.
- **Doctest**: the library example in [README.md](./README.md) is compiled and run.

To add a fixture (e.g. from a bug report), drop `tests/data/<name>.ics` in and generate the expectation: `cargo run --features cli -- template [--flags] tests/data/<name>.ics -o tests/data/<name>.<mode>.toml`. Add an empty `tests/data/<name>.lossy` if the source will not round-trip byte-for-byte (see [Known limitations](#known-limitations)). Real-world calendars belong here; prefer them over synthetic ones.

## Project layout

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
    util.rs              TOML/escape/calcard-value helpers
    datetime.rs          friendly date-times <-> iCalendar digits, offsets
    duration.rs          DURATION/TRIGGER <-> dotted duration.* keys
    recurrence.rs        RRULE <-> dotted recurrence.* keys
  edit.rs                module root for the format-preserving editor
  edit/
    tree.rs              Calendar/Component/Property/Container + Nodes DOM
    parse.rs             Parser: unfold + build the tree
    render.rs            fold content lines, detect end-of-line
tests/
  fixtures.rs            golden-fixture runner
  data/                  *.ics sources + *.toml expectations + *.lossy markers
```

## Architecture

tcal converts between a calcard `ICalendar` and an ergonomic TOML buffer, in two directions:

- **`project` / `project_with`** (read): turn an `ICalendar` into a fillable TOML scaffold. calcard is the reader; it validates and normalises values.
- **`apply` / `apply_with`** (write): fold an edited TOML buffer back onto the *original* iCalendar text.

The crucial design choice: **calcard is reader-only**. Its writer normalises folding, parameter casing and property order, so re-serialising churns the whole file. Instead `apply` patches the original text through `crate::edit`, a format-preserving editor (the `toml_edit` analog for iCalendar) that keeps every content line's original bytes and re-renders only the lines a modeled field actually changed. Its core invariant is `Calendar::parse(s).to_string() == s` for any input; on top of it, projecting then applying an untouched buffer reproduces the source byte-for-byte.

The modeled vocabulary lives in `template/model.rs`. A `Spec { key, name, fields, children }` describes one component type (`TOP_LEVEL` = event, todo, journal, free-busy, timezone; children are alarms and time-zone rules). Each `Field { key, name, hint, kind }` decouples the TOML `key` (friendly, e.g. `date-start`) from the iCalendar `name` (`DTSTART`). The `Kind` enum drives both directions per field: `Scalar`, `Enum` (lowercase in hints, uppercased on export), `Number`, `List`, `Date` (friendly value plus an adjacent `<key>-tz`), `CalAddress` (strips `mailto:`), `Offset` (`±HHMM`), `Attendee` (a `[[...]]` section with `display-name`/`value`/`role`/`status`), `Recur` and `Duration` (inline dotted `recurrence.*` / `duration.*` keys, each with a raw escape hatch).

Filtering (`project_with` / `apply_with` with selected type keys): no types selected projects the whole calendar; one type flattens at the document root; two or more keep the `VCALENDAR` root and show only those. Crucially, `apply` only reconciles the selected types, so a filtered edit never drops the unselected ones. `UID` and `DTSTAMP` are app-managed: not modeled, seeded for new events, preserved otherwise.

## Conventions

These are repo rules; follow them in new code.

- **`no_std`**: `#![no_std]` is unconditional, `extern crate std;` is gated on `feature = "cli"`. Every module imports the `alloc` items and macros it uses (`format`, `vec`, `String`, `ToString`, `Vec`, `ToOwned`); the `core`/`alloc` prelude does not include them. Import order: `core`, blank, `alloc` + `std`, blank, third-party, blank, `crate`. Use `crate::` paths, not `super::`.
- **No re-exports**: callers use module-qualified paths (`tcal::edit::tree::Calendar`); module roots only declare submodules, no `pub use`.
- **Structs over free functions** where there is a real receiver (see `Nodes`, `Parser`); keep genuinely stateless helpers as small free functions grouped by domain.
- **Comments**: every public module, function and type has a one-line doc; prose stays concise. Avoid bare inline `//` comments; when one is needed, tag it (`NOTE`, `HACK`, `SAFETY`). No em dashes; do not hard-wrap markdown.
- **Tests pin behaviour**: never adjust a test to fit the code; adjust the code to match correct, RFC-checked behaviour. Verify against RFC 5545, not model output.
- After any Rust change, run `cargo fmt` and keep `cargo clippy` clean for both the core and the CLI.

## Known limitations

These are deliberate (or pending) and explain the `.lossy` fixture markers:

- **RRULE canonicalisation**: calcard reorders `RRULE` tokens on read (canonical order: `FREQ, UNTIL, COUNT, INTERVAL, BYDAY, BYMONTHDAY, BYMONTH, BYSETPOS, WKST`), so a source rule in another order round-trips canonicalised, not byte-exact.
- **All-day `VALUE=DATE`**: an all-day date written without the parameter (`DTSTART:20220101`) is re-emitted RFC-correct (`DTSTART;VALUE=DATE:20220101`).
- **Attendee parameters**: only `CN`/`ROLE`/`PARTSTAT` are modeled; other parameters (`RSVP`, `CUTYPE`, ...) are dropped when an attendee line is rewritten.
- **List parameters**: `CATEGORIES`/`FREEBUSY` parameters (e.g. `FBTYPE`) are not modeled.

## Commit style

tcal follows the [conventional commits specification](https://www.conventionalcommits.org/en/v1.0.0/#summary). Keep the subject imperative and scoped; describe the *why* in the body when it is not obvious.
