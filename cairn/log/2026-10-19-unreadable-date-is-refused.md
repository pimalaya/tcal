---
cairn: log
change: unreadable-date-is-refused
landed: 2026-10-19
---

# A typed date nothing reads is refused, not written verbatim

`Field::check` in `src/template/model.rs` runs before each field is folded back: a date string that `parse_friendly_date` does not read, and that none of the property's original lines carries, fails with `TcalError::UnreadableDate`. `date-start = "martes 10:00"` now fails naming the key instead of writing `DTSTART:martes 10:00`; a calendar value the projection could not read, left untouched, still round-trips verbatim.

Capabilities moved: **template** (a typed date nothing reads is refused, added; a date typed as its hint is read, modified).
