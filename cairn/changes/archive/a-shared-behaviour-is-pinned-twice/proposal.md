---
cairn: change
id: a-shared-behaviour-is-pinned-twice
status: landed
created: 2026-08-29
---

# A shared behaviour is pinned in both tools or in neither

## Why

tCal and tCard are one design written twice, and two behaviours of the merge are already the same in both: a header note is wrapped to the column the document keeps, and a side that does not parse is refused by name rather than as a nameless failure. tCard gained a test for each today. tCal has both behaviours and a test for neither.

Behaviour pinned on one side of a pair only is behaviour the other side can lose in a refactor with nothing to report it, and it will be lost quietly: an unwrapped note still reads, and a refusal that names no side still refuses. Whichever tool happens to get attention is then the only one whose promises are kept, which is the opposite of writing the same design twice.

## What

Port both tests, adapted to the shapes tCal has: the merge is a struct with a `speaks_for` field rather than three arguments, and the unreadable side is refused as `ReadCalendar`. The wrapping law reads the document's header block, as tCard's does, since a note long enough to wrap is no longer one line to match.

Both are behaviours tCal already has, so the spec gains what it silently promised rather than the code gaining anything.
