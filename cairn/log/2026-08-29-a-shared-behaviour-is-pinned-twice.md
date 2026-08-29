---
cairn: log
change: a-shared-behaviour-is-pinned-twice
landed: 2026-08-29
---

# A shared behaviour is pinned in both tools or in neither

tCal wrapped its header notes and named the side of an unreadable input, and tested neither. tCard grew a law for each today, so the pair was pinned on one side only, which is how the two drift apart without a failing test anywhere.

## What landed

Two tests, no production code. `a_long_note_wraps_under_itself` joins the merge forcing laws: an unprojectable collision produces a note past the column, and the law reads the document's whole header block, holding that no line of it passes the column and that the note needing a second line gets an indented one. It runs the suite's own preamble check like every law beside it.

`an_unreadable_side_is_named` joins the merge unit tests: a merge handed a body that is not an iCalendar as its remote side is refused as `ReadCalendar` naming the remote side, rather than reported as a parse failure of nothing in particular.

Both behaviours were already here. What was missing was anything that would notice their leaving, and the spec now says what the code was already doing.

## Verification

Every configuration builds, clippy is clean over all targets, cargo fmt run. The merge forcing suite is 18 tests and the merge module 8, the whole suite green.

Capabilities moved: merge (ADDED: A read failure names the side it came from, A header note wraps at the document's column).
