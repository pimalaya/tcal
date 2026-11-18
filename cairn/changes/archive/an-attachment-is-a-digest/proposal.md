---
cairn: change
id: an-attachment-is-a-digest
status: landed
created: 2026-11-18
---

# An attachment is a digest

## Why

tCal kept `ATTACH` (RFC 5545 section 3.8.1.1) verbatim and unshown, so a form could neither see nor change an event's attachments. tCard settled the shape for its media values on 2026-09-29 (`an-embedded-photo-is-a-digest`, `logo-sound-and-key-are-media`), and its log named tCal's `ATTACH` as the owed twin; tCal's own `the-family-standards` left it out by name.

## What

**An `[[attach]]` table per line**, on events, to-dos and journals, holding exactly one of three keys, as tCard's `[[photo]]` does:

- **`embedded`**: an attachment the calendar carries inline, `ENCODING=BASE64;VALUE=BINARY` or a `data:` URI. `embedded.type` is its `FMTTYPE`, `embedded.sha256` the full digest of its bytes; the bytes are never shown. An unchanged digest folds back as the calendar's own line, wherever it stood; one the calendar does not hold is refused.
- **`file`**: a local path, read and inlined as `FMTTYPE=<sniffed>;ENCODING=BASE64;VALUE=BINARY`, every other parameter of the line it replaces kept.
- **`uri`**: a reference, kept as written; an unchanged one folds back as its own line too.

**The library still does no I/O**: `TcalTemplate::files` and `TcalMerged::files` carry the reader the caller hands it, the CLI's being the filesystem behind the usual path expansion; none refuses every `file`.

**Two new refusals**, both re-opening the editor: `embedded-unknown` and `file-unreadable`.

## Judged differently from tCard

iCalendar has one version and names a media type in `FMTTYPE`, so no legacy `TYPE` table and no text value. A merge needs no settling rule: ical-rs keeps both sides' attachments as a set, so there is no collision to offer or settle.

## What this is not

Not `VALARM`'s `ATTACH` (an alarm's sound or an email alarm's files), kept verbatim as before. Not an extraction command, which both twins still owe.
