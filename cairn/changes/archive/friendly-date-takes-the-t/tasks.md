---
cairn: tasks
change: friendly-date-takes-the-t
---

- [x] Split date and time on a space or `T` in `parse_friendly_date`, and read a trailing `Z` as UTC.
- [x] Tests: space, `T`, `Z`, a lone date, a value passed verbatim.
- [x] Checked by folding back a blank template with `date-end = "2026-10-20T10:30:00"`: `DTEND:20261020T103000` instead of the value verbatim.
- [x] `cargo test`, `cargo clippy --all-targets`, `cargo fmt`.
- [x] CHANGELOG: one `### Fixed` entry.
- [x] Fold delta.md into cairn/spec/template.md; append the log entry; mark `landed`.
