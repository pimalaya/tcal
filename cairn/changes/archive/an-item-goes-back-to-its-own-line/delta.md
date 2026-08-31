---
cairn: delta
change: an-item-goes-back-to-its-own-line
---

## ADDED Requirements

None.

## MODIFIED Requirements

### Requirement: A repeated property keeps its own line
*Folds into template.md.*

Where a component holds more than one property of a list field's name, folding a document back SHALL give each item to the line whose value it came out of, matched on the value the projection showed. An item no line held SHALL fill the room a line lost, in the order the document writes them, and an item past all of that room SHALL open a line of its own. A line left with no item SHALL be removed rather than take the items of another.

The form shows the items of every such line as one array, which is what makes the field editable at all. A line's parameters describe the items that line carried, so counting items off the front of the array instead hands them to whichever line has room: removing one item relabels every item behind it and drops the last line, on an edit that named neither.

#### Scenario: Two properties of one list name
- GIVEN a component holding two `CATEGORIES` properties, each carrying a different `LANGUAGE`
- WHEN an untouched projection is folded back
- THEN the component still holds two, each with the items and the parameter it had

#### Scenario: An item removed from the first of two lines
- GIVEN a component holding `FREEBUSY;FBTYPE=BUSY` with two periods beside `FREEBUSY;FBTYPE=FREE` with one
- WHEN the document is folded back with one busy period removed
- THEN the busy line carries the remaining busy period and the free line carries its own, unchanged

#### Scenario: An item renamed in place
- GIVEN a component holding one `CATEGORIES` line of two items
- WHEN the document is folded back with one item replaced by a value no line held
- THEN the line still carries two items and no second line is written

## REMOVED Requirements

None.
