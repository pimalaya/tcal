---
cairn: change
id: contest-by-identity
status: landed
created: 2026-08-29
---

# The document announces a conflict it does not contain

## Why

base holds Ada and Bob, both `NEEDS-ACTION`. Local drops Ada and accepts as Bob; remote declines as Bob. The merged document opens with

    # 1 conflict below is yours to decide, ...
    # this document cannot be applied until every one is decided.

and writes Bob's table once, `status = "ACCEPTED"`, with no duplicate key anywhere. The document parses, applies, and takes `ACCEPTED`. Bob's `DECLINED` is gone, and the reader was told to decide something the document never showed them.

This is worse than the silent loss it replaced. The merge is right: it reports the collision, and the preamble counts it. What fails is placing it. The report addresses an attendee by its calendar address, which is what ical-rs's `IcalPropPath::identity` was added for, while this crate still resolves it by `index`, the position among the component's same-named properties counted in the side that wrote it. Local removed Ada, so the position the report carries is Bob's in the base and the remote and nobody's in the merged calendar, whose only attendee sits at 0. The choice addresses a block that does not exist, no line ever contests it, and it is dropped.

Nothing reconciles the number the preamble announces with the number of contests written below it, which is why a dropped contest reaches a reader at all. The count is taken before the placement is attempted, and a placement that fails says nothing.

## What

Resolve a contested property by the identity the report carries, in each of the four calendars separately: the merged one, whose block the contest is written into, and the three the sides' lines are read from. A position is what tells same-named properties apart only where iCalendar gives them no identity, and it is only ever valid in the calendar it was counted in.

Reconcile the two numbers by construction. The preamble is written after the placement rather than before it, and announces the contests the document actually holds. A collision with no line to contest falls back to the header comment an unaddressable collision already becomes, saying the local value was kept, so nothing the merge reported is dropped without a word.

Assert the invariant directly, on every document the property suite generates, and generate the shapes that break it: a contested attendee, and a neighbouring removal on the local side.
