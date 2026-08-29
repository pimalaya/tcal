---
cairn: spec
capability: reading
status: current
---

# Reading

How a calendar is read, and what that reading is allowed to change. Every verb starts here: a body becomes a tree, the projection walks it, a merge reconciles two of them, and folding a document back writes onto it.

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
