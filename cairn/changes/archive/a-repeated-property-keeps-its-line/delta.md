---
cairn: delta
change: a-repeated-property-keeps-its-line
---

## ADDED Requirements

### Requirement: A repeated property keeps its own line
*Folds into template.md.*

Where a component holds more than one property of a list field's name, folding a document back SHALL write one line per property the component held, each taking as many items as it held, and an item past the last of them SHALL open a line of its own.

The form shows the items of every such line as one array, which is what makes the field editable at all. Writing that array back as one line collapses the properties into it, dropping every line past the first along with the parameters the form never showed, and it does so on a document nobody edited.

#### Scenario: Two properties of one list name
- GIVEN a component holding two `CATEGORIES` properties, each carrying a different `LANGUAGE`
- WHEN an untouched projection is folded back
- THEN the component still holds two, each with the items and the parameter it had

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
