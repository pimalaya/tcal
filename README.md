# tCal [![Documentation](https://img.shields.io/docsrs/tcal?style=flat&logo=docs.rs&logoColor=white)](https://docs.rs/tcal/latest/tcal) [![Matrix](https://img.shields.io/badge/chat-%23pimalaya-blue?style=flat&logo=matrix&logoColor=white)](https://matrix.to/#/#pimalaya:matrix.org) [![Mastodon](https://img.shields.io/badge/news-%40pimalaya-blue?style=flat&logo=mastodon&logoColor=white)](https://fosstodon.org/@pimalaya) [![Sponsor](https://img.shields.io/badge/sponsor-pink?style=flat&logo=github-sponsors&logoColor=white)](https://pimalaya.org/sponsor/)

Edit and merge [iCalendars](https://www.rfc-editor.org/rfc/rfc5545) as ergonomic TOML

```sh
tcal edit --event
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

This repository ships two interfaces: a Rust library projecting an iCalendar to TOML and back, and a CLI editing that projection in `$EDITOR`.

## Table of contents

- [Features](#features)
- [RFC coverage](#rfc-coverage)
- [Installation](#installation)
- [Usage](#usage)
- [AI policy](https://github.com/pimalaya/.github/blob/master/AI_POLICY.md)
- [License](#license)
- [Social](#social)
- [Contributing](./CONTRIBUTING.md)
- [Sponsoring](#sponsoring)

## Features

- **Projection** of a calendar into ergonomic TOML and back, backed by [ical-rs](https://github.com/pimalaya/ical).
- **Friendly** keys and values: cryptic property names become readable TOML keys, and a closed vocabulary lists what it accepts.
- **Structured** recurrence and duration: a rule or a length expands into named parts, with a raw escape hatch for the rest.
- **Discoverable** properties: every editable property is printed with an empty value, so the form is its own documentation.
- **Every component type**: events, to-dos, journals, free/busy reports and time zones, narrowed to the ones you ask for.
- **Minimal, lossless diffs**: only the lines you changed are re-rendered, and what tCal does not model is carried through verbatim.
- **Three-way merge**: what two sides both changed comes back as duplicate TOML keys, which do not parse until you decide them.
- **Slim library core**: the projection and the merge build without the standard library, the CLI living behind the opt-in `cli` feature.

## RFC coverage

| RFC    | What is covered                                                                                                                |
|--------|--------------------------------------------------------------------------------------------------------------------------------|
| [5545] | iCalendar: events, to-dos, journals, free/busy reports, time zones and alarms, their recurrence rules, durations and attendees |

[5545]: https://www.rfc-editor.org/rfc/rfc5545

## Installation

### Pre-built binary

As root:

```sh
curl -sSL https://raw.githubusercontent.com/pimalaya/tcal/master/install.sh | sudo sh
```

As a regular user:

```sh
curl -sSL https://raw.githubusercontent.com/pimalaya/tcal/master/install.sh | PREFIX=~/.local sh
```

These commands install the latest binary from the GitHub [releases](https://github.com/pimalaya/tcal/releases) section.

For a more up-to-date version, check the [releases](https://github.com/pimalaya/tcal/actions/workflows/releases.yml) workflow and look for the *Artifacts* section: those are built from `master`, with the default cargo features.

> [!NOTE]
> Pre-built binaries carry the `cli` feature and nothing else. If you need a different feature set, use another installation method.

### Cargo

The binary lives behind the `cli` feature, which is off by default so that a library consumer pays for none of it:

```sh
cargo install --locked --features cli tcal
```

The library alone is a `tcal` dependency, which pulls in none of that:

```sh
cargo add tcal
```

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

See documentation at [docs.rs](https://docs.rs/tcal/latest/tcal).

### CLI

Run `tcal --help` for the command tree, and `tcal <command> --help` for a command's arguments.

A few real command lines:

```sh
tcal template                              # a blank, fully documented form
tcal template event.ics                    # an existing calendar as TOML
tcal template - < event.ics                # the same, read from stdin
tcal template --event event.ics            # the event alone, flat at the root
tcal template --event --todo event.ics     # only events and to-dos, as blocks
tcal edit event.ics                        # edit in $EDITOR, written back in place
tcal edit - < event.ics > updated.ics      # edit a stream
tcal edit --output meeting.ics             # start a new event from scratch
tcal edit --editor "code --wait" event.ics # name the editor for one run
tcal merge base.ics local.ics remote.ics --output merged.ics
```

A type flag narrows the form and nothing else: a type it does not show is left exactly as it was when the result is written back.

The editor is the one `--editor` names, then `$VISUAL`, then `$EDITOR`, and nothing after those: tCal picks none of its own, and says so when neither variable is set. It reads no configuration file, so set them in your shell rc file. The command is spawned on the path of a temporary TOML file it edits in place, so it must block until the edit is done: use `code --wait`, not `code`.

A property `tcal merge` could not settle comes back written once per side, under the same TOML key:

```toml
# conflict, keep one line
# summary = "Standup" # base
summary = "Daily standup" # local
summary = "Team standup" # remote
```

TOML forbids duplicate keys, so an undecided document does not parse and nothing is written. Delete the line you do not want, or replace them all with a value of your own.

What the merge settled on its own is said in a comment at the head of the document instead, since offering a decided thing as a choice asks you to undo what you cannot see the reasons for.

Logs go to stderr, so they can be redirected to a file while the command output stays on stdout:

```sh
tcal template event.ics --log-level debug 2>/tmp/tcal.log
```

Use `--log-file <PATH>` to append them to a file directly. When `--log-level` is omitted the `RUST_LOG` environment variable is consulted, and `RUST_BACKTRACE=1` adds the full error backtrace.

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
