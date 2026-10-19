---
cairn: change
id: unreadable-date-is-refused
status: landed
created: 2026-10-19
---

# A typed date nothing reads is refused, not written verbatim

## Why

`friendly-date-takes-the-t` taught the reader the hint's own shape, but a date string still in no known form passes through verbatim: `date-start = "martes 10:00"` folds back to `DTSTART:martes 10:00`, which no iCalendar reader accepts, and nothing says so before the calendar server does. That was option 3 of #1, left as a follow-up.

The verbatim pass-through is not all wrong: the projection shows a calendar value it cannot read (`DTSTART:20260613T1400`) as the string the calendar wrote, so that it round-trips whole. Refusing every unreadable string would break that.

## What

- A date field holding a non-empty string that the friendly reader does not read, and that is not the value the calendar already carries for that property, fails the fold back with `TcalError::UnreadableDate`, naming the key and the value.
- The calendar's own unreadable value, left as projected, still round-trips verbatim.
- `apply` reports it as an error; `edit` names it and offers to re-edit, as it does for a document that does not parse.
