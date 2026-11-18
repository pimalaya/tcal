---
cairn: log
change: an-attachment-is-a-digest
landed: 2026-11-18
---

# An attachment is a digest

The `ATTACH` twin tCard's media values owed since 2026-09-29 (tcard's `an-embedded-photo-is-a-digest` and `logo-sound-and-key-are-media`). `ATTACH` was kept verbatim and unshown; events, to-dos and journals now project it as `[[attach]]` tables, each one of `embedded`, `file` or `uri`.

**src/template/media.rs**, tCard's media module cut to iCalendar: inline bytes are `ENCODING=BASE64` (with `VALUE=BINARY`) or a `data:` URI, their media type `FMTTYPE`; a `file` is written `FMTTYPE=<sniffed>;ENCODING=BASE64;VALUE=BINARY`, through tCard's sniffing table unchanged. No legacy `TYPE` table and no text value: iCalendar has one version and `FMTTYPE`. base64-codec and sha2 join the core, which the family-standards change had judged not needed while tCal decoded no base64.

**`Kind::Attach`** in the model, a sectioned kind beside the attendee. Its `params` are `ENCODING`, `VALUE` and `FMTTYPE`, so a `file` or `uri` replacing a line keeps every other parameter of it (`X-FILENAME`, `X-LABEL`). An embedded digest, and an unchanged `uri`, fold back as whichever line holds them, so reordering tables moves whole lines. Matching the `uri` by value is a step past tCard, whose positional pairing would patch a reordered URI onto the line that stood in its slot and give it that line's parameters; found by `reordered_attachments_keep_their_own_lines` here, still owed in tCard.

**The library still reads nothing**: `TcalTemplate::files` and `TcalMerged::files` carry a `TcalFiles` reader; none refuses every `file`. The CLI hands `edit`, `apply` and `merge` its `read_file` (`path_parser`, then `fs::read`). `content_lines` now returns a `TcalResult`; an empty block holding a refused value still counts as filled, so the refusal surfaces.

**Two refusals**: `TcalError::UnknownEmbedded` and `ReadFile`, coded `embedded-unknown` and `file-unreadable`, both document refusals the editor re-opens on.

**Merge**: ical-rs merges `ATTACH` lines as a set, so two sides replacing one attachment keep both, with no collision to offer or settle; tCard's settling rule has no tCal twin. Written down in the merge spec.

Not done: `VALARM`'s `ATTACH` (an alarm's sound, an email alarm's files), still verbatim; an extraction command, owed by both twins; sniffing PDF, which both tables read as `application/octet-stream`.

Capabilities moved: template (three requirements added), merge (one added), editor (re-open list), output (two codes), api (error enum wording).

Verified: fixtures regenerated, every one gaining empty `[[...attach]]` examples and nothing else; a new `attach.ics` fixture with RFC 5545's URI and CID examples and an inline text file round trips byte-exact, its digest equal to `sha256sum` of the decoded bytes; the CLI inlines a PNG from disk and fails coded `file-unreadable` and `embedded-unknown` under `--json`. The proptest's unmodelled `ATTACH` became `COMMENT`. 80 library tests and the integration suites green, clippy clean with and without features.
