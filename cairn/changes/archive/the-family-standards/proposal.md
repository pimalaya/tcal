---
cairn: change
id: the-family-standards
status: landed
created: 2026-11-14
---

# The family standards

## Why

tCal joins the 2026-11 Pimalaya family release beside Cardamum, Calendula and Comodoro, which took the shared kit (pimalaya-cli 0.4) and its output and error contract on 2026-11-12. tCal was still on pimalaya-cli 0.2 and ical-rs 0.5: under `--json` it printed the raw TOML or iCalendar, a failure carried no code, a refused command line printed nothing on stdout, and no command wrote the schemas of what it prints. tCard took the same change the same day (its cairn log 2026-11-14-the-family-standards), and the twins stay one design.

## What

**Dependencies.** ical-rs 0.7 and pimalaya-cli 0.4 (through a path patch to ../cli until the family pins it), `rust-version` 1.89, every dependency declared with `default-features = false` and the features it uses, clap without `wrap_help` and `env`.

**Output.** A verb prints its document as a `DocumentOutput`: in text the document itself, so pipes keep working; under `--json` `{contents}`, or the receipt `{path, size}` when it went to a file, which prints nothing in text since tCal runs as the composer of programs piping their own stdout.

**Failures.** A binary-local `ErrorCode` (`calendar-unreadable`, `template-unreadable`, `collision-undecided`, `date-unreadable`, `component-unknown`, `editor-missing`, `editor-failed`), given to a library failure from the `TcalError` it carries and raised by the editor round trip; `ErrorReport::eval_coded` prints it. Arguments go through `pimalaya_cli::clap::parse`, so a refused command line exits 2 coded `usage-invalid`; `apply - -` fails coded the same.

**Schemas.** `json-schema` writes one schema per verb and `tcal-error`; a test checks the registry against the command tree; the package installs them under share/schemas.

**ical-rs folds.** An edited line longer than 75 octets now goes out folded (ical-rs 0.6 onwards); the template spec and the lib header say so.

## Judged not applicable

No REPL and no MCP server: tCal holds no account or connection a session would keep open, and its verbs open an editor. No configuration, so no `ConfigFlags`, no account flag, no welcome and no shipped sample. No events and no partial outcome (exit 3): every verb produces one document or fails. No `Columns`, `Fields` or `sanitize::text`: tCal prints no record or list, only the document it was asked for, as data. No base64-codec and no CPU token: tCal decodes no base64 and ical-rs is built without its codecs.

## What this is not

Not the owed `ATTACH` twin of tCard's media kinds, and not a version bump.
