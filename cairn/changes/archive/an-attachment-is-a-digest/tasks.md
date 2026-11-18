---
cairn: tasks
change: an-attachment-is-a-digest
---

# Tasks

- [x] media.rs: read inline bytes and references, inline a file, digest
- [x] `Kind::Attach` on event, to-do and journal, projected as `[[attach]]` tables
- [x] `TcalTemplate::files` and `TcalMerged::files`, the CLI's `read_file` handed to `edit`, `apply` and `merge`
- [x] `UnknownEmbedded` and `ReadFile`, coded `embedded-unknown` and `file-unreadable`, both document refusals
- [x] Tests: digest round trip, reorder, file, uri, unknown digest, no reader, new event, merge; an `attach.ics` fixture
- [x] Fixtures regenerated, the proptest's unmodelled `ATTACH` swapped for `COMMENT`
- [x] Spec folded, log written, CHANGELOG
