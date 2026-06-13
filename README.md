# tcal [![Documentation](https://img.shields.io/docsrs/tcal?style=flat&logo=docs.rs&logoColor=white)](https://docs.rs/tcal/latest/tcal) [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya)

CLI and lib to edit calendar events ([iCalendar](https://www.rfc-editor.org/rfc/rfc5545) `VEVENT`) as ergonomic TOML: the TOML calendar, à la [jCal](https://www.rfc-editor.org/rfc/rfc7265).

iCalendar is already plain text, so there is nothing to compress; what hurts is its crypticness (date-times like `20260613T140000`, `;TZID=` parameters, opaque `PARTSTAT`/`ROLE` codes) and the sheer number of properties nobody remembers. tcal projects an iCalendar into a commented, prefilled TOML scaffold you edit in `$EDITOR`, then folds your edits back onto the original calendar. Cryptic property names are given friendly TOML keys (`DTSTART` becomes `date-start`, `RRULE` becomes `recurrence`, an attendee's `CN`/`PARTSTAT` become `display-name`/`status`). By default every component type is shown as a `[[block]]` (events, to-dos, journals, free/busy, time zones), with the actual instances filled in and an empty example for each absent type, so the scaffold doubles as documentation; you keep what you need. Per-type flags (`--event`, `--todo`, ...) narrow the view: one flag flattens that type at the root, several show only those types, and on save the types you did not select are left untouched (so `--todo` on an event adds a to-do without disturbing the event). Date-times become a friendly `2026-06-13 14:00` with the time zone on its own line.

This repository ships two layers:

- Low-level **library** projecting between a [calcard](https://crates.io/crates/calcard) `ICalendar` and TOML: `project` emits the whole-calendar scaffold (`[[block]]` per component), `project_with` narrows it to a chosen set of types (one flattened at the root, several filtered under the `VCALENDAR`), and `apply` / `apply_with` detect the buffer's shape and patch it back onto the original text, reconciling only the selected types while carrying every unmodeled property (the app-managed `UID` and `DTSTAMP`, custom `X-*`) and every unselected or unmodeled component type over verbatim.
- High-level **CLI** with two verbs: `template` prints the TOML scaffold (blank or prefilled), `edit` runs the full "project to `$EDITOR` to apply" round-trip and emits the resulting iCalendar.

## Table of contents

- [Features](#features)
- [Installation](#installation)
  - [Pre-built binary](#pre-built-binary)
  - [Cargo](#cargo)
  - [Nix](#nix)
  - [Sources](#sources)
- [Usage](#usage)
  - [Library](#library)
  - [CLI](#cli)
- [FAQ](#faq)
- [License](#license)
- [AI disclosure](#ai-disclosure)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- **iCalendar event to TOML projection**, backed by [calcard](https://crates.io/crates/calcard) (RFC 5545 parser and writer).
- **Friendly date-times**: the cryptic `20260613T140000` becomes `2026-06-13 14:00`, all-day events are `2026-06-13`, UTC values end in ` UTC`, and the time zone (`TZID`) moves onto its own `date-start-tz` key. Every date-time key is prefixed `date-` (`date-start`, `date-end`, `date-due`, ...).
- **Friendly keys**: cryptic property names get readable TOML keys (`date-start`, `date-end`, `offset-from`, an attendee's `display-name`/`status`); enumerated values are listed lowercase in the hints (`confirmed, tentative, cancelled`) and uppercased to their canonical form on export; numeric properties (`priority`, `percent`, `repeat`) are plain numbers.
- **Structured recurrence**: the dense `RRULE` is broken into dotted `recurrence.*` keys (`recurrence.frequency`, `recurrence.interval`, `recurrence.count`, `recurrence.until`, `recurrence.by-day`, `recurrence.by-month`, `recurrence.by-month-day`, `recurrence.by-position`, `recurrence.week-start`), with `until` shown as a friendly date and a raw `recurrence.rule = "..."` escape hatch for rules using parts tcal does not model, so nothing is lost.
- **Structured duration**: a `DURATION` (and an alarm `TRIGGER` offset) is broken into dotted `duration.week`/`duration.day`/`duration.hour`/`duration.min`/`duration.sec` magnitudes. The sign is implied by context (a trigger fires before the event, so it is negative), so you only fill the amounts; a value that is not a plain duration falls back to a raw `duration.raw = "..."` key.
- **Grouped, discoverable form**: every modeled property is listed and empty (an empty value is ignored, like a removed line), prefilled when present, with a comment only where the value is not self-evident. Fields cluster by shape, separated by blank lines: the bare scalar keys (`summary`/`description` leading), then the dates, the duration, and the recurrence, each its own group. Inline comments are tab-aligned to a single column across the whole block, so they all sit at the same level. Attendees expand into a `[[attendee]]` block (`display-name` first, then `value`, `role`, `status`) carrying their accepted values inline; new events are seeded with a fresh `UID` and `DTSTAMP`.
- **Whole calendar by default, filtered on demand**: by default every component type is a repeatable `[[block]]` (`event`, `todo`, `journal`, `free-busy`, `timezone`, with alarms and time-zone rules nested like `[[event.alarm]]`) — actual instances filled in, an empty example for each absent type, so the scaffold doubles as a menu. Add, edit, or drop a block to add, change, or remove a component (an empty block is ignored). The per-type flags cumulate: `tcal template --event` flattens a single event at the root (no `[[event]]` ceremony), `--event --todo` shows just those two as blocks, and a flag filter only ever touches the types it shows, so editing through it adds to a calendar without removing the rest.
- **Minimal diff, lossless for everything unmodeled**: `apply` patches the original text through a format-preserving editor, re-rendering only the lines you changed. Properties tcal does not list (the app-managed `UID` and `DTSTAMP`, `SEQUENCE`, custom `X-*`), other components (`VTIMEZONE`), folding, casing and ordering are all kept byte-for-byte. The TOML is an editing affordance, not an interchange format, so `apply` always works against the original calendar.
- **Two verbs, no subcommand maze**: `template` always emits TOML, `edit` always emits an iCalendar; `SOURCE` resolves deterministically (`-` is stdin, an existing file is read, otherwise literal iCalendar contents, and omitting it starts a blank template).

> [!TIP]
> tcal is written in [Rust](https://www.rust-lang.org/) and uses [cargo features](https://doc.rust-lang.org/cargo/reference/features.html) to gate the CLI. The default feature set is declared in [Cargo.toml](./Cargo.toml).

## Installation

### Pre-built binary

The CLI binary `tcal` can be installed from the latest [GitHub release](https://github.com/pimalaya/tcal/releases) using the install script:

*As root:*

```sh
curl -sSL https://raw.githubusercontent.com/pimalaya/tcal/master/install.sh | sudo sh
```

*As a regular user:*

```sh
curl -sSL https://raw.githubusercontent.com/pimalaya/tcal/master/install.sh | PREFIX=~/.local sh
```

For a more up-to-date version, check out the [pre-releases](https://github.com/pimalaya/tcal/actions/workflows/pre-releases.yml) GitHub workflow: pick the latest run and grab the artifact matching your OS. These are built from the `master` branch.

> [!NOTE]
> Pre-built binaries are built with the default cargo features. If you need a different feature set, use another installation method.

### Cargo

```sh
cargo install tcal --locked --features cli
```

You can also use the git repository for a more up-to-date (but less stable) version:

```sh
cargo install --locked --git https://github.com/pimalaya/tcal.git
```

To use `tcal` as a library, add it to your `Cargo.toml`:

```toml
[dependencies]
tcal = { version = "0.0.1", default-features = false }
```

Dropping the default `cli` feature gives a slim library build with no clap, no editor integration: just the `project` / `apply` projection over a calcard `ICalendar`.

### Nix

If you have the [Flakes](https://nixos.wiki/wiki/Flakes) feature enabled:

```sh
nix profile install github:pimalaya/tcal
```

Or run without installing:

```sh
nix run github:pimalaya/tcal -- template < event.ics
```

### Sources

```sh
git clone https://github.com/pimalaya/tcal
cd tcal
nix run
```

## Usage

### Library

Project a calendar event to TOML, then fold edits back:

```rust,ignore
use tcal::{ical, template};

let calendar = ical::parse(input)?;

// Emit the whole-calendar scaffold ([[block]] per component).
// (project_with(&calendar, &["event".into()])? narrows to chosen types.)
let scaffold = template::project(&calendar);

// ... user edits `scaffold` in an editor ...

// Fold the edits back onto the original text: only changed lines are
// re-rendered, everything else stays byte-for-byte identical.
let updated = template::apply(input, &edited)?;
```

### CLI

Print a blank, fully-documented template:

```sh
tcal template
```

Project an existing event to TOML (path, stdin via `-`, or literal contents):

```sh
tcal template event.ics
tcal template - < event.ics
tcal template --event event.ics              # just the event, flattened
tcal template --event --todo event.ics       # only events and to-dos
```

Edit an event in `$EDITOR`. With a file source, the result is written back in place; otherwise it goes to stdout (or `--output`):

```sh
tcal edit event.ics
tcal edit - < event.ics > updated.ics
tcal template | $EDITOR /dev/stdin   # inspect the scaffold first
```

Start a new event from scratch and write it out:

```sh
tcal edit --output meeting.ics
```

## FAQ

<details>
  <summary>Which calendar component does tcal edit?</summary>

  All of them, as `[[blocks]]`: `event`, `todo`, `journal`, `free-busy`, `timezone` (with nested `[[event.alarm]]`, `[[timezone.standard]]`/`[[timezone.daylight]]`). Every type is listed (actual instances filled, an empty example for each absent type); repeated components show as repeated blocks. The per-type flags narrow the view: one (`--event`) flattens just that type at the root, several (`--event --todo`) show only those as blocks, and a filtered edit only touches the types it shows (so the rest of the calendar is preserved on save). Component types tcal does not model, and unmodeled properties, are kept verbatim but not surfaced.
</details>

<details>
  <summary>How do I write dates and times?</summary>

  Use `YYYY-MM-DD HH:MM` for a timed event (`2026-06-13 14:00`), `YYYY-MM-DD` alone for an all-day event, and append ` UTC` for a UTC value. For a zoned time, set the adjacent `date-start-tz` / `date-end-tz` key to an IANA zone like `Europe/Paris`; leave it empty for UTC or floating time. A raw iCalendar value (`20260613T140000`) is accepted too.
</details>

<details>
  <summary>How does `tcal edit` pick the editor?</summary>

  The [edit](https://crates.io/crates/edit) crate resolves `$VISUAL` first, then `$EDITOR`, then an OS default. tcal does not expose a config override: set `VISUAL` / `EDITOR` in your shell rc file.
</details>

<details>
  <summary>Will tcal reformat my whole calendar on edit?</summary>

  No. `apply` patches the original text through a format-preserving editor (the iCalendar analog of toml_edit): only the lines of modeled fields you actually changed are re-rendered, so the diff is minimal. Folding, parameter casing, property order and line endings of every untouched line are kept byte-for-byte.
</details>

<details>
  <summary>What happens to properties and components tcal does not list?</summary>

  They are kept verbatim. The scaffold surfaces the modeled component vocabulary, but `apply` carries every unmodeled property (`DTSTAMP`, `SEQUENCE`, custom `X-*`) and every unmodeled component type straight from the original calendar into the result. Unmodeled properties inside an edited component are likewise preserved (removing a whole block, of course, removes the component and everything in it).
</details>

<details>
  <summary>How to debug the CLI?</summary>

  Use `--log <level>` where `<level>` is one of `off`, `error`, `warn`, `info`, `debug`, `trace`:

  ```sh
  tcal --log trace template event.ics
  ```

  The `RUST_LOG` environment variable, when set, overrides `--log` and supports per-target filters (see the [env_logger](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) documentation). `RUST_BACKTRACE=1` enables full error backtraces. Logs are written to `stderr`.
</details>

## License

This project is licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.

## AI disclosure

This project is developed with AI assistance. This section documents how, so users and downstream packagers can make informed decisions.

- **Tools**: Claude Code (Anthropic), Opus 4.8, invoked locally with a persistent project-scoped memory and a small set of repo-specific rules.

- **Used for**: Refactors, mechanical multi-file edits, boilerplate (feature gates, error enums, derive macros, trait impls), test scaffolding, doc polish, exploratory design conversations.

- **Not used for**: Engineering, critical code, git manipulation (commit, merge, rebase…), real-world tests.

- **Verification**: Every AI-assisted change is read, compiled, tested, and formatted before commit (`nix develop --command cargo check / cargo test / cargo fmt`). Behavioural correctness is verified against the relevant RFC or upstream spec, not assumed from the model output. Tests are never adjusted to fit AI-generated code; the code is adjusted to fit correct behaviour.

- **Limitations**: AI models occasionally produce code that compiles and passes tests but is subtly wrong: off-by-one errors, missed edge cases, plausible but nonexistent APIs, stale RFC references. The verification workflow catches most of this; it does not catch all of it. Bug reports are welcome and taken seriously.

- **Last reviewed**: 13/06/2026

## Social

- Chat on [Matrix](https://matrix.to/#/#pimalaya:matrix.org)
- News on [Mastodon](https://fosstodon.org/@pimalaya) or [RSS](https://fosstodon.org/@pimalaya.rss)
- Mail at [pimalaya.org@posteo.net](mailto:pimalaya.org@posteo.net)

## Sponsoring

[![nlnet](https://nlnet.nl/logo/banner-160x60.png)](https://nlnet.nl/)

Special thanks to the [NLnet foundation](https://nlnet.nl/) and the [European Commission](https://www.ngi.eu/) that have been financially supporting the project for years:

- 2022 → 2023: [NGI Assure](https://nlnet.nl/project/Himalaya/)
- 2023 → 2024: [NGI Zero Entrust](https://nlnet.nl/project/Pimalaya/)
- 2024 → 2026: [NGI Zero Core](https://nlnet.nl/project/Pimalaya-PIM/)
- *2027 in preparation…*

If you appreciate the project, feel free to donate using one of the following providers:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/soywod)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/soywod)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/soywod)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
