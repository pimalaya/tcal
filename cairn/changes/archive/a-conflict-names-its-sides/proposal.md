---
cairn: change
id: a-conflict-names-its-sides
status: landed
created: 2026-08-31
---

# The merge reads a conflict right before left

## Why

`IcalMergeConflict` was `{ right, reason }`, so the only place tCal touches it read `sides.read(&conflict.right, &conflict.reason)`, and `Sides::read` took the remote action before the local one. Neither order is anybody's choice: the struct's field order picked it.

ical-rs has since renamed the field to `left` and put it first, so a conflict reads `{ left, right }` as vcard-rs's twin does. This is the tCal half of that change.

## What

- Read `conflict.left` and `conflict.right`, in that order.
- Take the same two in that order in `Sides::read`, naming the parameters after the sides the library names, and keep the local and remote vocabulary for what the arms bind.
