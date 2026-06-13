# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Added the `project` / `apply` projection library between a calcard `ICalendar` and an ergonomic TOML buffer.

  `project` emits a fillable TOML form from the first `VEVENT`, listing the modeled vocabulary; fields are uncommented and empty (an empty value is ignored, like a removed line), prefilled when present, and carry an inline `# e.g. ...` hint only where the value is not self-evident. Cryptic date-times become a friendly `2026-06-13 14:00` (with all-day, UTC and a broken-out `dtstart_tz` time-zone key), and attendees expand into `value` / `cn` / `role` / `partstat` blocks. `UID` and `DTSTAMP` are not modeled: they are app-managed (seeded for new events, preserved otherwise) and cannot be set through the buffer. `apply` rebuilds modeled fields from the edited buffer and carries every unmodeled property (`UID`, `DTSTAMP`, `SEQUENCE`, custom `X-*`) and every sibling component (`VALARM`, `VTIMEZONE`) over verbatim, since the TOML is an editing affordance rather than an interchange format.

- Added the `tcal` CLI with two verbs.

  `template [SOURCE]` prints the TOML scaffold (blank or prefilled). `edit [SOURCE]` runs the full "project to `$EDITOR` to apply" round-trip and emits the resulting iCalendar, writing a file source back in place. `SOURCE` resolves deterministically: `-` reads stdin, an existing file is read, otherwise the value is treated as literal iCalendar contents, and omitting it starts from a blank template. New (sourceless) events are seeded with a fresh `urn:uuid` v4 `UID` and a current `DTSTAMP`.
