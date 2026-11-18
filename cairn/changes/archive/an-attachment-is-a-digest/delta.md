---
cairn: change
id: an-attachment-is-a-digest
status: landed
created: 2026-11-18
---

# Delta

## ADDED Requirements

### Requirement: An inline attachment is shown as its digest
An `ATTACH` whose bytes the calendar carries, under `ENCODING=BASE64` or as a `data:` URI, SHALL project as `embedded.type`, its `FMTTYPE` lowercased when it declares one, and `embedded.sha256`, the full SHA-256 of its decoded bytes in lowercase hex, and never as those bytes. Any other value SHALL project as `uri`, as the calendar wrote it.

#### Scenario: An inline attachment round trips untouched
- GIVEN an event holding `ATTACH;FMTTYPE=text/plain;ENCODING=BASE64;VALUE=BINARY:Zm9v`
- WHEN it is projected
- THEN the form shows `embedded.type = "text/plain"` and the digest `printf foo | sha256sum` prints, and no base64
- AND WHEN that form is folded back
- THEN the calendar comes back byte-exact

### Requirement: An unchanged attachment comes back as its own line
A table carrying `embedded`, or a `uri` some line already holds, SHALL fold back as that line, byte for byte, wherever it stood. A digest no `ATTACH` line holds SHALL be refused, naming it.

#### Scenario: Two attachments swapped
- GIVEN an event holding an inline attachment then a referenced one, each with parameters of its own
- WHEN their tables are swapped
- THEN the two lines swap, each keeping its own parameters

### Requirement: A file is inlined as base64 binary
A table carrying `file` SHALL fold back as the bytes that file holds, written `FMTTYPE=<type>;ENCODING=BASE64;VALUE=BINARY`, the type read from the bytes as tCard reads them. The line it replaces SHALL keep every parameter but `ENCODING`, `VALUE` and `FMTTYPE`; a `uri` replacing bytes drops those three likewise.

The library SHALL NOT read the file itself: the template SHALL carry a reader its caller provides, and a template without one SHALL refuse every `file`, as SHALL a reader failing, naming the path. Where one table carries several keys, `file` SHALL win over `uri`, and `uri` over `embedded`.

#### Scenario: A PNG replaces a text attachment
- GIVEN an inline attachment carrying `X-FILENAME=foo.txt`
- WHEN its `embedded` is swapped for `file = "~/me.png"`
- THEN the line reads `ATTACH;FMTTYPE=image/png;X-FILENAME=foo.txt;ENCODING=BASE64;VALUE=BINARY:...`, and projecting it again shows it as `embedded`

### Requirement: Both sides' attachments are kept
A merge where both sides replaced an attachment SHALL keep both new ones and offer no choice: ical-rs merges `ATTACH` lines as a set, so the document shows two `[[attach]]` tables and folds back as the merged calendar.

## MODIFIED Requirements

### Requirement: The editor re-opens on a document it cannot fold
`edit` and `merge` SHALL offer to re-open the editor on a refusal of the edited document, which the reader can fix by editing it again: TOML that does not parse, a collision left undecided, a date that reads as none, a digest the calendar does not hold, a file that cannot be read. They SHALL NOT offer it on any other failure: a calendar that does not read, an editor that failed or exited with a failure, a temporary file that cannot be read back. tCard follows the same rule over its own refusals.

### Requirement: A failure carries a stable code
The codes gain `embedded-unknown` (the document names a digest the calendar does not hold) and `file-unreadable` (a file the document names cannot be read).

## REMOVED Requirements

None.
