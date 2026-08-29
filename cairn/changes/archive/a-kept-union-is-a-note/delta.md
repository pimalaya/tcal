---
cairn: delta
change: a-kept-union-is-a-note
---

## ADDED Requirements

### Requirement: A union is said in the header
Where both sides edited the items of one multi-valued property, the document SHALL say so in its header comment, stating that the items of both were kept. It SHALL NOT contest them.

The items of such a value merge as a set, RFC 5545 giving them no order, so both sides' additions and removals all apply and nothing collides. That is the right outcome: two sides each adding a category should keep both, and putting them to a reader would throw one away for no reason. The silence is what is wrong, since the merged value is then one neither side wrote and nobody was told it was assembled.

#### Scenario: Both sides rewrite a list
- GIVEN a base holding `CATEGORIES:a,b`, a local holding `CATEGORIES:c,d` and a remote holding `CATEGORIES:e,f`
- WHEN they are merged
- THEN the calendar holds all four, the header says the items of both were kept, and the document applies as it stands

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
