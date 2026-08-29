---
cairn: change
id: parse-once
status: landed
created: 2026-08-29
---

# Two libraries read every calendar, and the second one loses bytes

## Why

tcal depends on calcard and on ical-rs at once, and every body goes through both. A merge parses the three sides with ical-rs, runs the reconciliation, serialises the result, and calcard then parses that result again so the projection has a typed calendar to walk. One body, two readers, two models.

The second read is where a confirmed defect enters, and it is not fixable from here. A comma-separated list item loses exactly one space after an escape, so `CATEGORIES:a\,  b` comes back `a\, b`. It happens inside calcard 0.3.13 before the projection sees anything, and `template::project` is handed an already-parsed calendar rather than the source bytes. The fold-back side does now hold the original line, so the loss could be hidden on an untouched value, but the value shown to the reader in the document would still be wrong, which papers over the symptom and keeps the lie. It is recorded as tcard-tcal-escape-eats-a-space, with a reproduction the suite carries ignored.

The editor exists for the same reason: calcard is a normalising reader and writer, churning line folding, parameter casing and property order even where nothing changed, so tcal keeps every content line's original bytes itself. That is the same design as ical-rs's tree layer, maintained next door, fuzzed, and now carrying the merge and its identity addressing as well.

Nothing calcard supplies is missing from ical-rs, which carries the model, components, properties, parameters, values, the byte-faithful CST, recurrence, timezones, validation, jCal and JSCalendar. It covers more than calcard does here, not less.

## What

- Drop the calcard dependency and project from ical-rs's own model.
- Retire src/edit in favour of ical-rs's tree layer rather than maintaining a second implementation of the same idea.
- Read each body once, so the merge and the projection agree by construction instead of by round trip.
- The escape reproduction closes because the reader that lost those bytes is gone. Un-ignore it.

## Order

After ical-rs's identity change is released, so the port targets the merge that landed rather than the one it replaced. tcal goes after tcard, following the shape tcard settles on, as merge and prefer-local already did. The two crates should end up with the same projection architecture, since their divergences have cost real behaviour twice already.

## What it costs, honestly

template/model.rs is written against calcard's typed model and is the real work; the rest is deletion. tcal's model is the larger of the two, carrying components, recurrence and zoned dates, so confirm ical-rs's decoded model covers every property the projection shows before committing. A gap would surface as a silently dropped field rather than a compile error.

The golden fixture corpus is the safety net. Projection equality and byte-exact round-trip must hold across it before and after. Watch the zoned dates in particular: the multi-line contested value and the `-tz` arm are the places where a model swap is most likely to move bytes without moving behaviour.
