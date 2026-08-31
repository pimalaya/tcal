---
cairn: delta
change: a-participation-status-is-the-components
---

## ADDED Requirements

### Requirement: A component-dependent vocabulary is hinted per component
*Folds into template.md.*

Where RFC 5545 closes a vocabulary differently per component, the hint a block writes SHALL be the one its own component defines. That covers `STATUS` (3.8.1.11) and an attendee's `PARTSTAT` (3.2.12): a to-do also accepts `COMPLETED` and `IN-PROCESS`, and a journal accepts neither those nor `TENTATIVE` and `DELEGATED`.

A blank form is the documentation of what a field takes, so a hint listing another component's values is worse than none: it names values the component does not define and hides the ones it does.

#### Scenario: The participation statuses of a to-do and a journal
- GIVEN a blank projection of the whole calendar
- WHEN the attendee blocks are read
- THEN `[[todo.attendee]]` offers `completed` and `in-process`, `[[journal.attendee]]` offers neither those nor `tentative` and `delegated`, and `[[event.attendee]]` offers the event's five

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
