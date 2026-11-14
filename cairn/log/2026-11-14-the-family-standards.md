---
cairn: log
change: the-family-standards
landed: 2026-11-14
---

# tCal took the family standards

The twin of tCard's change of the same day. Cargo.toml moved to ical-rs 0.7 and pimalaya-cli 0.4 (normal and build dependencies, `[patch.crates-io]` to ../cli in place of the 55b6693 git pin), `rust-version` 1.89, every dependency with `default-features = false` and named features (clap without `env` and `wrap_help`, with `error-context`, `help`, `suggestions` and `usage`; chrono `clock` and `std`; toml_edit `parse` and `display`; uuid `std` and `v4`), and schemars and serde under `cli`. ical-rs 0.6 and 0.7 needed no code change; pimalaya-cli 0.4 moved the completion and manual commands under `clap::commands::completion` and `clap::commands::manual`.

src/cli/output.rs holds `DocumentOutput`, an untagged `Printed {contents}` or `Saved {path, size}`, which replaces the `Output` writer of src/cli/args.rs; the four verbs print it. src/cli/error.rs holds `ErrorCode` and `coded`, applied by `Command::execute`; src/cli/editor.rs raises `editor-missing` and `editor-failed` as `CodedError`s at the root of their chains. `apply - -` fails through `error::usage_error`, coded `usage-invalid`, exit 1. src/main.rs parses with `pimalaya_cli::clap::parse` and evaluates with `ErrorReport::<ErrorCode>::eval_coded`. src/cli/json_schema.rs registers the four verbs and `tcal-error`; package.nix installs the schemas under share/schemas.

ical-rs, from 0.6 on, folds an edited line at 75 octets, where tCal wrote it on one line: checked with the binary on an edited `SUMMARY`. The template spec and the lib header's known limitations say so.

Tests: tests/output.rs (nine, the binary under `--json` and in text) and `an_edited_long_line_is_folded` in tests/projection.rs failed on the old code first, run from a `git archive HEAD` copy. deny.toml took Comodoro's `[graph] targets` and lost the unused CC0-1.0.

Checks: `cargo fmt --check`; `cargo clippy --all-targets -- -D warnings` with default features, `--all-features` and `--no-default-features`; `cargo test --all-features`; `cargo deny check` (advisories, bans, licences and sources ok; warnings: syn 2 beside syn 3 through pimalaya-cli's inquire and crossterm, and `allow-org.github` unmatched while the patch is a path).

Capabilities: output created (five requirements); template (An edited long line is folded ADDED).
