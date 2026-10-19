---
cairn: change
id: friendly-date-takes-the-t
status: landed
created: 2026-10-16
---

# A date typed as the hint shows it is read, not written verbatim

## Why

The blank template offers every date as an empty string, `date-end = ""`, with the hint `# 2026-06-13T14:30:00` beside it. Typing the hint's shape between the quotes is the obvious way to fill it, but the friendly reader splits a date from its time on whitespace only, so `"2026-10-20T10:30:00"` is not recognised and passes through verbatim: the fold-back writes `DTEND;TZID=Europe/Madrid:2026-10-20T10:30:00`, which no iCalendar reader accepts (RFC 5545 3.3.5 wants `20261020T103000`). Nothing warns; the calendar server is the first to refuse it.

A TOML datetime literal, unquoted, already works, which is what a prefilled template shows; only the blank one leads to the string.

## What

- The friendly reader splits the date from the time on a space or on `T` (RFC 3339 5.6), so the hint's own shape is read.
- A time ending in `Z` reads as UTC, as the ` UTC` suffix does.
- A value still not in the friendly form passes through verbatim, as before.
