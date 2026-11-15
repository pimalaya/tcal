# Migration guide

## From v0.2 to v0.3

v0.3 takes the output and error contract the Pimalaya CLIs share. The CHANGELOG lists every change; these are the ones to act on.

### The document

- **Long lines are folded.** An edited line longer than 75 octets goes out folded, as RFC 5545 section 3.1 asks, where v0.2 wrote it on one line. A script comparing a calendar byte for byte against an unfolded one unfolds both first.
- **A date tCal cannot read is refused**, naming its key, where v0.2 wrote it verbatim into a property no iCalendar reader accepts. Write it like `2026-06-13T14:30:00`; a date in iCalendar form (`20260613T140000Z`) is still accepted.

### JSON output and exit codes

- **A document prints as `{"contents": ...}`** under `--json`, where v0.2 printed the raw TOML or iCalendar, and as the receipt `{"path": ..., "size": ...}` when it went to a file. Text output is unchanged: the document as itself, nothing for a file.
- **A failure prints the failure object** under `--json`, with a stable `code` (`calendar-unreadable`, `template-unreadable`, `collision-undecided`, `date-unreadable`, `component-unknown`, `usage-invalid`, or `null`), and exits 1. Read `code` rather than matching the message.
- **A refused command line exits 2**, printing that object coded `usage-invalid` under `--json`: a mistyped flag, `apply - -` (one stdin cannot carry both inputs), and `edit` or `merge` under `--json`.
- **`edit` and `merge` no longer run under `--json`**: the editor drew on the stdout a JSON caller reads. A program wanting JSON projects the form with `tcal --json template`, edits it itself, and folds it back with `tcal --json apply`.
- `tcal json-schema` writes the schema of what each command prints; packages install them under share/schemas.

### Building

- The minimum supported Rust version is 1.89.
- The binary is the `cli` feature, on by default: `cargo install tcal` builds it, and a library consumer turns it off with `default-features = false`.
