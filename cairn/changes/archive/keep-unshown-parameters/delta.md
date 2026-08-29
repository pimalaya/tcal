---
cairn: delta
change: keep-unshown-parameters
---

## ADDED Requirements

### Requirement: Deciding a collision keeps what the document never showed
Folding an edited document back SHALL keep every parameter of a modelled property that the projection does not show, in the place it held on the line. A parameter the projection does show SHALL be the document's: taken from the edited value, and dropped where the document cleared it.

The projection is an editing affordance rather than an interchange format, so what it does not show it does not own. A line is patched rather than rebuilt: the value and the shown parameters come from the document, the rest of the line is the calendar's own. Rebuilding dropped `RSVP`, which decides whether a scheduling client asks an attendee at all, and `SENT-BY`, which is who may speak for the organiser, so settling a conflict by hand quietly undid what the merge refuses changes to protect.

A property's lines are paired with the document's by position, which is how the projection shows them in the first place.

#### Scenario: An unshown parameter survives a decision
- GIVEN a merged document whose attendee carries a parameter the form has no key for
- WHEN the document is applied
- THEN the attendee line still carries it

#### Scenario: A shown parameter is the document's to clear
- GIVEN a merged document with an attendee's status emptied
- WHEN the document is applied
- THEN the status parameter is gone and the rest of the line is unchanged
