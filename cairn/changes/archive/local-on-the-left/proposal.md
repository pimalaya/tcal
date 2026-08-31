---
cairn: change
id: local-on-the-left
status: landed
created: 2026-08-30
---

# Put the local calendar on the merge's left

## Why

tcal put its local calendar on the merge's right and then asked for the right side to be preferred, so that a collision would still keep the local value. It did that because authority could only be claimed for the right side: ical-rs named a side rather than a role.

Every other caller of a Pimalaya merge puts local on the left, where the merged bytes are its own and it wins a collision by default. One field held tcal apart from that, and the field is gone.

## What

Move the local calendar to the left and let the default preference carry it, and drop `merge --speaks-for` along with the refusal it bought.

Done when local is the left side, the preference is the default one, the flag is gone, and the spec and the reader-facing prose say so.

## Consequence

A change to a property someone else organises is no longer refused: ical-rs does not judge authority any more. Where both sides add to one list, the local items now lead, the merged bytes being the local side's.
