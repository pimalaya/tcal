---
cairn: log
change: a-destination-is-a-flag
date: 2026-08-31
---

# The merge's fourth path is told from the other three by counting

`merge` took its destination as a fourth positional. `tcal merge a.ics b.ics c.ics d.ics` is four paths of one shape, where three are read and one is overwritten, and only their order says which is which. The one destructive argument in the whole CLI was the one nothing named.

It is now `-o`/`--output`, which is what `template` and `edit` already used and what tCard's `merge` used. The three inputs stay positional: a merge is meaningless without all three, and base-local-remote is the order every merge tool takes them in.

The usage lines are now the same in both crates:

    Usage: tcard merge [OPTIONS] --output <PATH> <BASE> <LOCAL> <REMOTE>
    Usage: tcal merge [OPTIONS] --output <PATH> <BASE> <LOCAL> <REMOTE>

Timing was the whole reason to do it now rather than later. tCal has no tag, so the surface is still free; one release on, the same change is a breaking one.

Capabilities moved: merge.
