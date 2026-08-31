---
cairn: change
id: a-destination-is-a-flag
status: landed
created: 2026-08-31
---

# The merge's fourth path is told from the other three by counting

## Why

`merge` took its destination as a fourth positional, so `tcal merge a.ics b.ics c.ics d.ics` gave four paths of one shape and only their order said which three are read and which one is written. Miscount and the merge overwrites an input. Nothing in the line says which position is the destructive one.

The other verbs already answer this. `template` and `edit` write through `-o`/`--output`, and tCard's `merge` does too, so tCal's was the one verb in either crate where the destination was positional.

This is the last of the surface an unreleased crate can still change freely. After a tag it is semver.

## What

- Take the destination as `-o`/`--output`, keeping the three inputs positional.
- Say in the spec why the three are positional and the fourth is not.
