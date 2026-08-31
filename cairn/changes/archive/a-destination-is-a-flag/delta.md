---
cairn: delta
change: a-destination-is-a-flag
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: Merging is a verb over three files
*Folds into merge.md.*

`merge` SHALL take a base, a local and a remote calendar as its three positional paths, and the path to write as `-o`/`--output`, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

The three inputs are positional because a merge is meaningless without all three and their order is the one every merge tool uses. The destination is a flag because it is the one path that is written rather than read, and a fourth positional beside three inputs is told apart by counting. tCard spells it the same way.

#### Scenario: The destination is named
- GIVEN a merge invoked with its three calendars
- WHEN the path to write is given
- THEN it is given as `-o`/`--output` rather than as a fourth positional

## REMOVED Requirements

None.
