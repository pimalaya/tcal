---
cairn: delta
change: a-conflict-names-its-sides
---

## ADDED Requirements

### Requirement: A conflict is read left before right
*Folds into merge.md.*

Where a merge conflict is read, the left side SHALL be taken before the right one, matching the order the report names them in.

The left side is the local calendar, whose bytes the merged calendar is built from and whose value a collision keeps. Reading it first is the order everything else in the merge already states, and taking the remote action first only ever reflected a field order the library has since corrected.

#### Scenario: Reading one conflict
- GIVEN a conflict the merge reported
- WHEN it is read against the four calendars
- THEN its left side is taken first, and the reading is the same note or choice either way

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
