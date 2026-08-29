---
cairn: delta
change: parse-once
---

## ADDED Requirements

### Requirement: One reader per body

A body SHALL be read once. The reader that parses a calendar for the merge SHALL be the reader that parses it for the projection, so the two agree by construction rather than by serialising between them.

Two readers do not merely cost a parse. They disagree, and the disagreement is invisible: a value the first reads faithfully and the second normalises reaches the document already changed, and no test comparing the document against the second reader's output can see it.

The reader SHALL be byte-faithful, so what the projection does not touch is what the calendar already held, and folding a document back SHALL rewrite only the lines whose modelled value the document moved.

#### Scenario: A value no reader normalises
- GIVEN a calendar whose list item carries an escape
- WHEN it is projected and applied unchanged
- THEN it comes back byte-exact

#### Scenario: A calendar the file holds beside the one being read
- GIVEN a file holding several calendars
- WHEN the first is projected and applied
- THEN the others are still there, byte for byte

## MODIFIED Requirements

### Requirement: Merging is a verb over three files
`merge` SHALL take a base, a local and a remote calendar as paths, plus the path to write, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

It SHALL take the calendar address the edited side speaks for as an option, passed through to the merge, and SHALL claim nothing where it is not given.

Taking the three rather than a pre-merged body with markers is what keeps the document a calendar. Line markers are how a line-oriented merge shows an unresolved region, and an iCalendar is not lines: a marker in one would break every parser downstream, including this one. The merge is a pure function over bodies already at hand, so running it here rather than receiving its output costs nothing and invents no format.

The merged calendar SHALL be projected as the merge produced it, rather than written out and read back. Serialising between the reconciliation and the document is a second reading of the same body, and a byte the merge preserved that a second reading changed would reach the reader as the merge's own work.

The local side is the merge's right side: the edited one, whose actions are replayed onto the remote side's bytes and on whose behalf authority is claimed. It SHALL also be the preferred side, so a collision the merge does not settle holds the local value in the merged bytes, which the document then asks about rather than keeps quietly. The two are separate statements: being replayed is what makes the local side judgeable, and being preferred is what stops that judgement costing it every collision. This is what tCard does with local on the left, where a card has no organiser and nothing to judge.

#### Scenario: The output is written only when the document is decided
- GIVEN a merge whose document still holds an undecided collision
- WHEN the editor exits
- THEN the output path is not written

#### Scenario: Being judged does not cost the collision
- GIVEN a local side speaking for an attendee, changing a property both sides changed
- WHEN the merge is projected
- THEN the merged bytes carry the local value, and a change the attendee has no authority over is still refused

## REMOVED Requirements

None.
