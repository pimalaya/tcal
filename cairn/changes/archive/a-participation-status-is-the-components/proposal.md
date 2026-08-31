---
cairn: change
id: a-participation-status-is-the-components
status: landed
created: 2026-08-31
---

# A to-do's attendee is offered an event's statuses

## Why

`PARTSTAT` is not one vocabulary. RFC 5545 3.2.12 gives a to-do two statuses an event does not have, `COMPLETED` and `IN-PROCESS`, and gives a journal three where an event has five, dropping `TENTATIVE` and `DELEGATED`. The attendee block writes one hint for all of them, the event's:

    status = ""    # needs-action, accepted, declined, tentative, delegated

So a reader filling in a `[[todo.attendee]]` is not shown the two statuses that block is for, and a reader filling in a `[[journal.attendee]]` is shown two the journal grammar does not define. The blank form is the documentation, which is precisely why the wrong vocabulary in it costs something.

`STATUS` has the same shape and already gets this right, each component spec carrying its own hint. The attendee block does not, because it writes its keys from one function that knows the entry and not the component around it.

## What

Carry the participation statuses on the attendee field, the way every other vocabulary is carried on the field that shows it, and let the block write the ones its own component defines.
