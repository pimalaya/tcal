---
cairn: log
change: a-participation-status-is-the-components
date: 2026-08-31
---

# A to-do's attendee was offered an event's statuses

`PARTSTAT` is not one vocabulary. RFC 5545 section 3.2.12 gives a to-do `COMPLETED` and `IN-PROCESS` on top of the event set, and gives a journal three where an event has five. The attendee block wrote the event's list in all of them, so a `[[todo.attendee]]` never showed the two statuses that block is for and a `[[journal.attendee]]` showed two the journal grammar does not define.

`STATUS` already got this right, each component spec carrying its own hint. The attendee block did not, because it wrote its keys from one function that knew the entry and not the component around it.

The attendee field now carries its statuses, the way every other vocabulary is carried on the field that shows it, and the projection and the merge both read them from there. A free/busy attendee takes the event's set, RFC 5545 defining none of its own.

Capabilities moved: template.
