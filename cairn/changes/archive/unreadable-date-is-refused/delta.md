---
cairn: delta
change: unreadable-date-is-refused
---

## ADDED Requirements

### Requirement: A typed date nothing reads is refused
A date field given as a string that is neither in the friendly form, nor in the iCalendar digit form (`20260613T140000Z`), nor the value the calendar already carries for that property SHALL fail the fold back, naming the key and the value. The calendar's own value SHALL still pass through verbatim.

Written verbatim, such a string makes a property no iCalendar reader accepts, and the calendar server is the first to say so. The calendar's own value is the exception because the projection shows a form it cannot read as the string the calendar wrote, so that it round-trips whole.

#### Scenario: A date typed in words
- GIVEN an event template whose `date-start` is set to `"next tuesday"`
- WHEN it is folded back
- THEN it fails naming `date-start` and `"next tuesday"`

#### Scenario: A date pasted in iCalendar form
- GIVEN an event template whose `date-start` is set to `"20260613T140000Z"`
- WHEN it is folded back
- THEN the event carries `DTSTART:20260613T140000Z`

#### Scenario: The calendar's own unreadable value
- GIVEN an event whose `DTSTART` is `20260613T1400`, projected as `date-start = "20260613T1400"`
- WHEN the projection is folded back unchanged
- THEN the event comes out byte for byte as it went in

## MODIFIED Requirements

### Requirement: A date typed as its hint is read
A date field given as a string SHALL be read when its date and time are separated by a space or by `T`, and a time ending in `Z` or followed by ` UTC` SHALL read as UTC. A string in neither form SHALL be refused unless it is the value the calendar already carries (see *A typed date nothing reads is refused*).
