---
cairn: tasks
change: unreadable-date-is-refused
---

- [x] `TcalError::UnreadableDate { key, value }`.
- [x] `Field::check` refuses a date string the friendly reader does not read and no original line carries; `apply_component` and `reconcile` return a `TcalResult`.
- [x] `edit` names the key and value and offers to re-edit, as for a TOML syntax error.
- [x] Tests: a date typed in words is refused; the calendar's own unreadable value round-trips, and edited into another unreadable one is refused.
- [x] Checked with `tcal apply`: `date-start = "martes 10:00"` fails naming the key, where v0.2.0 wrote `DTSTART:martes 10:00`; with `edit`, the prompt to re-edit, then `DTSTART:20261020T100000` once fixed.
- [x] `cargo test`, `cargo clippy --all-targets`, `cargo fmt`, build without default features.
- [x] CHANGELOG: one `### Changed` entry.
- [x] Fold delta.md into cairn/spec/template.md; append the log entry; mark `landed`.
