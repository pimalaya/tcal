---
cairn: delta
change: parse-once
---

## ADDED Requirements

### Requirement: One reader per body

A body SHALL be read once. The reader that parses a calendar for the merge SHALL be the reader that parses it for the projection, so the two agree by construction rather than by serialising between them.

Two readers do not merely cost a parse. They disagree, and the disagreement is invisible: a value the first reads faithfully and the second normalises reaches the document already changed, and no test comparing the document against the second reader's output can see it.

#### Scenario: A value no reader normalises
- GIVEN a calendar whose list item carries an escape
- WHEN it is projected and applied unchanged
- THEN it comes back byte-exact

## MODIFIED Requirements

### Requirement: Merging is a verb over three files
Unchanged in what it requires of the verb. What changes is that the merged calendar is no longer serialised and re-read before it is projected: the projection walks what the merge produced, so a byte the merge preserved is a byte the document is built from.

## REMOVED Requirements

None.
