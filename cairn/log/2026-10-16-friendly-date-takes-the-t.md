---
cairn: log
change: friendly-date-takes-the-t
landed: 2026-10-16
---

# A date typed as the hint shows it is read, not written verbatim

`parse_friendly_date` in `src/template/datetime.rs` splits the date from the time on a space or on `T`, and reads a trailing `Z` as UTC beside the ` UTC` suffix. A blank template filled with `date-end = "2026-10-20T10:30:00"` now folds back to `DTEND:20261020T103000` instead of the value verbatim; a value in neither form still passes through.

Capabilities moved: **template** (a date typed as its hint is read, added).
