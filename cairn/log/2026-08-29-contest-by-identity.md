---
cairn: log
change: contest-by-identity
date: 2026-08-29
---

# The document announces a conflict it does not contain

base held Ada and Bob. Local dropped Ada and accepted as Bob, remote declined as Bob, and the merged document opened with `1 conflict below is yours to decide`, wrote Bob's table once with `status = "ACCEPTED"`, and applied without a murmur. The reader was told the document could not be applied until they decided something the document never showed them, and following the instruction lost Bob's `DECLINED`.

The merge was right. ical-rs reported the collision and named the attendee by calendar address, which is what `IcalPropPath::identity` landed for. This crate still read the path's `index`, the position counted in the side that wrote the action, and used it in all four calendars. Ada's removal moved Bob to 0 in the merged one and left him at 1 in the base and the remote, so the choice addressed an attendee block that did not exist, no line ever contested it, and it was dropped. The preamble had already counted it.

## What landed

- **A contested property is resolved by identity, per calendar.** `index_of` takes the report's identity where there is one and finds the entry carrying that calendar address in whichever calendar is being read, falling back to the counted position only where iCalendar gives the property no identity. Both uses of the raw index are gone: the merged block a contest is written into, and the lines read from the base, the local and the remote.

- **The preamble is written after the placement.** `decorate` renders the body first and keeps the header aside, so the count it announces is the number of contests the document holds rather than the number the merge reported. A choice that found no line to contest falls back to `Choice::kept`, the header comment an unaddressable collision already becomes, so nothing reported is dropped in silence. The two numbers are now one number by construction.

## Verification

- `a_removal_does_not_swallow_a_neighbours_collision` is no longer ignored, and now pins the whole shape: one attendee table, `ACCEPTED` and `DECLINED` contested in it, the refusal naming `status`, and keeping the remote side yielding `PARTSTAT=DECLINED` with Ada still gone.
- `announces_what_it_holds` asserts the invariant on every document the property suite produces, and the generators were widened to the shapes that break it: a contested attendee status, and a generated neighbouring removal on the local side. Reverting the identity resolution fails six of the seventeen tests rather than one.
- The whole suite green: 66 lib, 16 merge forcing (1 still ignored, blocked in the merge crate), 5 projection laws, the fixture database, the doctest. `--all-features`, `--no-default-features` and `--no-default-features --features merge` all build, `clippy --all-features --all-targets` is clean, `cargo fmt` run.

Capabilities moved: merge (ADDED: The document holds every conflict it announces; MODIFIED: A nested collision stays inside its table).
