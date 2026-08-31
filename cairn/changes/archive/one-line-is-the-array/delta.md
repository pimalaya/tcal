---
cairn: change
id: one-line-is-the-array
status: landed
created: 2026-08-31
---

# Delta

## ADDED Requirements

### Requirement: One line leaves nothing to disambiguate
A list field holding at most one line SHALL take the array as that line's items, in the order the document wrote them. There is no second line to attribute an item to, so an added item joins the line and the parameters it carries.

That is the spelling the README documents, `categories = ["pimalaya", "cli"]` for `CATEGORIES:pimalaya,cli`, and it is what someone editing a single-line property means by adding to it.

#### Scenario: A second item added to a lone line
- GIVEN a component holding one `CATEGORIES` property, or none
- WHEN the document is folded back with a second item in the array
- THEN the component holds one `CATEGORIES` property carrying both items

## MODIFIED Requirements

### Requirement: A repeated property keeps its own line
*Folds into template.md.*

Where a component holds more than one property of a list field's name, folding a document back SHALL give each item to the line whose value it came out of, matched on the value the projection showed. An item no line held SHALL fill the room a line lost, in the order the document writes them, and whatever is left over SHALL share one new line between them. A line left with no item SHALL be removed rather than take the items of another.

The form shows the items of every such line as one array, which is what makes the field editable at all. A line's parameters describe the items that line carried, so counting items off the front of the array instead hands them to whichever line has room: removing one item relabels every item behind it and drops the last line, on an edit that named neither.

One new line rather than one each: which line's parameters a leftover item should carry is the question several lines make unanswerable, so they carry none, together.

#### Scenario: Two properties of one list name
- GIVEN a component holding two `CATEGORIES` properties, each carrying a different `LANGUAGE`
- WHEN an untouched projection is folded back
- THEN the component still holds two, each with the items and the parameter it had

#### Scenario: An item removed from the first of two lines
- GIVEN a component holding `FREEBUSY;FBTYPE=BUSY` with two periods beside `FREEBUSY;FBTYPE=FREE` with one
- WHEN the document is folded back with one busy period removed
- THEN the busy line carries the remaining busy period and the free line carries its own, unchanged

#### Scenario: Two items added beside two lines
- GIVEN a component holding two `CATEGORIES` properties, each carrying a different `LANGUAGE`
- WHEN the document is folded back with two items no line held
- THEN the two lines keep their items and their parameters, and one new `CATEGORIES` carries both added items

## REMOVED Requirements
