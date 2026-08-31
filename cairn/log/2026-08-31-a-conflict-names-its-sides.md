---
cairn: log
change: a-conflict-names-its-sides
date: 2026-08-31
---

# The merge reads a conflict left before right

ical-rs renamed `IcalMergeConflict::reason` to `left` and made it the first field, so a conflict now reads `{ left, right }` the way vcard-rs's twin does. tCal touched that struct in one place, and it read right before left because the field order said so.

`Sides::read` now takes the left side then the right. Its parameters are named after the sides the library names, and the match arms keep binding `local`, which is what the left side is here: the calendar the merged bytes come from and the one a collision keeps. The doc comment that had to explain which half of the pair carried which side is gone, the two names saying it.

Nothing about the reading changed. The same conflicts become the same notes and the same choices, and every merge law still holds.

Capabilities moved: merge.
