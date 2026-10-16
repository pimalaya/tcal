---
cairn: delta
change: friendly-date-takes-the-t
---

## ADDED Requirements

### Requirement: A date typed as its hint is read
A date field given as a string SHALL be read when its date and time are separated by a space or by `T`, and a time ending in `Z` or followed by ` UTC` SHALL read as UTC. A string in neither form SHALL pass through verbatim.

The blank template offers a date as an empty string beside a hint written with `T`, so filling the quotes the way the hint shows is the first thing a reader does; writing that value verbatim produces a property no iCalendar reader accepts, and nothing says so before the calendar server does.

#### Scenario: The hint's shape between the quotes
- GIVEN a blank event template whose `date-end` is set to `"2026-10-20T10:30:00"` and `date-end-tz` to `"Europe/Madrid"`
- WHEN it is folded back
- THEN the event carries `DTEND;TZID=Europe/Madrid:20261020T103000`
