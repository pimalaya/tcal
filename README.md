# tCal [![Documentation](https://img.shields.io/docsrs/tcal?style=flat&logo=docs.rs&logoColor=white)](https://docs.rs/tcal/latest/tcal) [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya) [![Sponsor](https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white)](https://pimalaya.org/sponsor/)

CLI & lib to edit [iCalendars](https://www.rfc-editor.org/rfc/rfc5545) as ergonomic TOML.

```sh
$ tcal edit --event
```

```toml
summary = "Check for tcal issues"
categories = ["pimalaya", "cli"]
url = "https://github.com/pimalaya/tcal/issues"
organizer = "pimalaya.org@posteo.net"
class = "public"
priority = 5
status = "confirmed"
recurrence.frequency = "daily"
recurrence.interval = 1

[[attendee]]
display-name = "Pimalaya"

[[alarm]]
summary = "Go check daily tcal issues"
action = "display"
trigger.min = 5
```

Output:

```ics
BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//Pimalaya//tcal//EN
BEGIN:VEVENT
UID:1f34e439-ca07-446f-af28-f5b7d3afcfc8
DTSTAMP:20260613T211938Z
SUMMARY:Check for tcal issues
CATEGORIES:pimalaya,cli
URL:https://github.com/pimalaya/tcal/issues
ORGANIZER:mailto:pimalaya.org@posteo.net
CLASS:PUBLIC
PRIORITY:5
STATUS:CONFIRMED
RRULE:FREQ=DAILY;INTERVAL=1
BEGIN:VALARM
SUMMARY:Go check daily tcal issues
ACTION:DISPLAY
TRIGGER:-PT5M
END:VALARM
END:VEVENT
END:VCALENDAR
```

This repository ships two interfaces:

- Rust **library** to generate iCalendar from/to TOML projection
- **CLI** to print and/or edit TOML template using `$EDITOR`

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
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [Contributing](./CONTRIBUTING.md)
- [Social](#social)
- [Sponsoring](#sponsoring)

## Features

- Partial `no_std` support
- iCalendar from/to TOML **projection**, backed by [calcard](https://crates.io/crates/calcard) (RFC 5545).
- **Friendly** keys and values: cryptic names become readable TOML keys.
- **Structured** recurrence and duration.
- **Discoverable** properties: prints all available properties with empty values by default, fill the ones you need.
- **Minimal, lossless diffs**: `apply` patches the original text through a format-preserving editor, re-rendering only the lines you changed.
- **Three-way merge**, backed by [ical-rs](https://crates.io/crates/ical-rs): what two sides both changed comes back as duplicate TOML keys, which do not parse until you decide them.

## Installation

### Pre-built binary

tcal is not yet released, therefore the only way to get a pre-built binary is to check out the [releases](https://github.com/pimalaya/tcal/actions/workflows/releases.yml) GitHub workflow and look for the *Artifacts* section.

> [!NOTE]
> Such binaries are built with the default cargo features. If you need specific features, please use another installation method.

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
tcal = "0.0.1"
```

The library has no default features: it is a slim `no_std` (plus `alloc`) build with no clap, no editor integration, just the `project` / `apply` projection over a calcard `ICalendar`. The three-way merge behind `tcal merge` is the opt-in `merge` feature, the only one that links a second iCalendar library ([ical-rs](https://crates.io/crates/ical-rs)). The CLI lives behind the opt-in `cli` feature (enabled above with `cargo install --features cli`), which pulls `merge` in.

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

```rust
use tcal::{ical, template};

let input = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nSUMMARY:Lunch\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
let calendar = ical::parse(input).unwrap();

// Project the whole calendar to a TOML scaffold ([[block]] per component).
// (project_with(&calendar, &["event".to_owned()]) narrows to chosen types.)
let scaffold = template::project(&calendar);
assert!(scaffold.contains("summary = \"Lunch\""));

// After the user edits the scaffold, fold it back onto the original text:
// only changed lines are re-rendered, everything else stays byte-for-byte.
let edited = scaffold.replace("Lunch", "Team lunch");
let updated = template::apply(input, &edited).unwrap();
assert!(updated.contains("SUMMARY:Team lunch"));
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

Merge two divergent calendars against their common ancestor, decide what the merge could not, and write the result:

```sh
tcal merge base.ics local.ics remote.ics merged.ics
tcal merge base.ics local.ics remote.ics merged.ics --speaks-for me@example.com
```

A property both sides changed comes back written once per side:

```toml
# conflict, keep one line
# summary = "Standup" # base
summary = "Daily standup" # local
summary = "Team standup" # remote
```

TOML forbids duplicate keys, so such a document cannot be applied: delete the lines you do not want, or replace them all with a value of your own. The output file is written only once the edited document parses.

## FAQ

### Which calendar components does tcal edit?

All of them, as `[[blocks]]`: `event`, `todo`, `journal`, `free-busy`, `timezone` (with nested `[[event.alarm]]`, `[[timezone.standard]]`/`[[timezone.daylight]]`). Every type is listed (actual instances filled, an empty example for each absent type); repeated components show as repeated blocks. The per-type flags narrow the view: one (`--event`) flattens just that type at the root, several (`--event --todo`) show only those as blocks, and a filtered edit only touches the types it shows (so the rest of the calendar is preserved on save). Component types tcal does not model, and unmodeled properties, are kept verbatim but not surfaced.

### How do I write dates and times?

Dates are native TOML values. Write a bare date (`2026-06-13`) for an all-day event, a local date-time (`2026-06-13T14:00:00`) for a timed one, and a `Z` offset (`2026-06-13T14:00:00Z`) for a UTC value. For a zoned time, keep the local date-time and set the adjacent `date-start-tz` / `date-end-tz` key to an IANA zone like `Europe/Paris`; that key only appears for a named zone (UTC and floating times need none). The older `YYYY-MM-DD HH:MM[ UTC]` string form is still accepted too.

### Why does `tcal merge` show some conflicts as comments rather than as a choice?

Because the merge already decided them, and offering a decided thing as a choice asks you to undo something you cannot see the reasons for. Three settle themselves: a removal against an update, where the update wins whichever side it came from; a recurrence rule changed on one side while an overriding instance moved on the other, where both survive and you are only being warned that the rule may have moved the ground the instance stood on; and a change refused because `--speaks-for` says you are an attendee rather than the organiser of that meeting (RFC 5546 section 3.2). A fourth is a comment for a different reason: where a collision lands on something the document does not show, there is no key to write twice, so the comment says that the local value was kept. Everything else, where two values are genuinely in the running, is written as duplicate keys.

### How does `tcal edit` pick the editor?

The [edit](https://crates.io/crates/edit) crate resolves `$VISUAL` first, then `$EDITOR`, then an OS default. tcal does not expose a config override: set `VISUAL` / `EDITOR` in your shell rc file.

### Will tcal reformat my whole calendar on edit?

No. `apply` patches the original text through a format-preserving editor (the iCalendar analog of toml_edit): only the lines of modeled fields you actually changed are re-rendered, so the diff is minimal. Folding, parameter casing, property order and line endings of every untouched line are kept byte-for-byte.

### What happens to properties and components tcal does not list?

They are kept verbatim. The scaffold surfaces the modeled component vocabulary, but `apply` carries every unmodeled property (`DTSTAMP`, `SEQUENCE`, custom `X-*`) and every unmodeled component type straight from the original calendar into the result. Unmodeled properties inside an edited component are likewise preserved (removing a whole block, of course, removes the component and everything in it).

### How do I debug the CLI?

Use `--log <level>` where `<level>` is one of `off`, `error`, `warn`, `info`, `debug`, `trace`:

```sh
tcal --log trace template event.ics
```

The `RUST_LOG` environment variable, when set, overrides `--log` and supports per-target filters (see the [env_logger](https://docs.rs/env_logger/latest/env_logger/#enabling-logging) documentation). `RUST_BACKTRACE=1` enables full error backtraces. Logs are written to `stderr`.

## License

This project is licensed under either of:

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

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
- 2026 → 2027: [NGI Zero Commons Fund](https://nlnet.nl/project/Pimalaya-pimdir/)

This program is part of Pimalaya, free software funded entirely by grants and donations. If you find it useful, consider [sponsoring](https://pimalaya.org/sponsor/) its development:

[![GitHub](https://img.shields.io/badge/-GitHub%20Sponsors-fafbfc?logo=GitHub%20Sponsors)](https://github.com/sponsors/soywod)
[![Ko-fi](https://img.shields.io/badge/-Ko--fi-ff5e5a?logo=Ko-fi&logoColor=ffffff)](https://ko-fi.com/pimalaya)
[![Buy Me a Coffee](https://img.shields.io/badge/-Buy%20Me%20a%20Coffee-ffdd00?logo=Buy%20Me%20A%20Coffee&logoColor=000000)](https://www.buymeacoffee.com/pimalaya)
[![Liberapay](https://img.shields.io/badge/-Liberapay-f6c915?logo=Liberapay&logoColor=222222)](https://liberapay.com/pimalaya)
[![thanks.dev](https://img.shields.io/badge/-thanks.dev-000000?logo=data:image/svg+xml;base64,PHN2ZyB3aWR0aD0iMjQuMDk3IiBoZWlnaHQ9IjE3LjU5NyIgY2xhc3M9InctMzYgbWwtMiBsZzpteC0wIHByaW50Om14LTAgcHJpbnQ6aW52ZXJ0IiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxwYXRoIGQ9Ik05Ljc4MyAxNy41OTdINy4zOThjLTEuMTY4IDAtMi4wOTItLjI5Ny0yLjc3My0uODktLjY4LS41OTMtMS4wMi0xLjQ2Mi0xLjAyLTIuNjA2di0xLjM0NmMwLTEuMDE4LS4yMjctMS43NS0uNjc4LTIuMTk1LS40NTItLjQ0Ni0xLjIzMi0uNjY5LTIuMzQtLjY2OUgwVjcuNzA1aC41ODdjMS4xMDggMCAxLjg4OC0uMjIyIDIuMzQtLjY2OC40NTEtLjQ0Ni42NzctMS4xNzcuNjc3LTIuMTk1VjMuNDk2YzAtMS4xNDQuMzQtMi4wMTMgMS4wMjEtMi42MDZDNS4zMDUuMjk3IDYuMjMgMCA3LjM5OCAwaDIuMzg1djEuOTg3aC0uOTg1Yy0uMzYxIDAtLjY4OC4wMjctLjk4LjA4MmExLjcxOSAxLjcxOSAwIDAgMC0uNzM2LjMwN2MtLjIwNS4xNTYtLjM1OC4zODQtLjQ2LjY4Mi0uMTAzLjI5OC0uMTU0LjY4Mi0uMTU0IDEuMTUxVjUuMjNjMCAuODY3LS4yNDkgMS41ODYtLjc0NSAyLjE1NS0uNDk3LjU2OS0xLjE1OCAxLjAwNC0xLjk4MyAxLjMwNXYuMjE3Yy44MjUuMyAxLjQ4Ni43MzYgMS45ODMgMS4zMDUuNDk2LjU3Ljc0NSAxLjI4Ny43NDUgMi4xNTR2MS4wMjFjMCAuNDcuMDUxLjg1NC4xNTMgMS4xNTIuMTAzLjI5OC4yNTYuNTI1LjQ2MS42ODIuMTkzLjE1Ny40MzcuMjYuNzMyLjMxMi4yOTUuMDUuNjIzLjA3Ni45ODQuMDc2aC45ODVabTE0LjMxNC03LjcwNmgtLjU4OGMtMS4xMDggMC0xLjg4OC4yMjMtMi4zNC42NjktLjQ1LjQ0NS0uNjc3IDEuMTc3LS42NzcgMi4xOTVWMTQuMWMwIDEuMTQ0LS4zNCAyLjAxMy0xLjAyIDIuNjA2LS42OC41OTMtMS42MDUuODktMi43NzQuODloLTIuMzg0di0xLjk4OGguOTg0Yy4zNjIgMCAuNjg4LS4wMjcuOTgtLjA4LjI5Mi0uMDU1LjUzOC0uMTU3LjczNy0uMzA4LjIwNC0uMTU3LjM1OC0uMzg0LjQ2LS42ODIuMTAzLS4yOTguMTU0LS42ODIuMTU0LTEuMTUydi0xLjAyYzAtLjg2OC4yNDgtMS41ODYuNzQ1LTIuMTU1LjQ5Ny0uNTcgMS4xNTgtMS4wMDQgMS45ODMtMS4zMDV2LS4yMTdjLS44MjUtLjMwMS0xLjQ4Ni0uNzM2LTEuOTgzLTEuMzA1LS40OTctLjU3LS43NDUtMS4yODgtLjc0NS0yLjE1NXYtMS4wMmMwLS40Ny0uMDUxLS44NTQtLjE1NC0xLjE1Mi0uMTAyLS4yOTgtLjI1Ni0uNTI2LS40Ni0uNjgyYTEuNzE5IDEuNzE5IDAgMCAwLS43MzctLjMwNyA1LjM5NSA1LjM5NSAwIDAgMC0uOTgtLjA4MmgtLjk4NFYwaDIuMzg0YzEuMTY5IDAgMi4wOTMuMjk3IDIuNzc0Ljg5LjY4LjU5MyAxLjAyIDEuNDYyIDEuMDIgMi42MDZ2MS4zNDZjMCAxLjAxOC4yMjYgMS43NS42NzggMi4xOTUuNDUxLjQ0NiAxLjIzMS42NjggMi4zNC42NjhoLjU4N3oiIGZpbGw9IiNmZmYiLz48L3N2Zz4=)](https://thanks.dev/u/gh/soywod)
[![PayPal](https://img.shields.io/badge/-PayPal-0079c1?logo=PayPal&logoColor=ffffff)](https://www.paypal.com/paypalme/soywod)
