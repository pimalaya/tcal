---
cairn: change
id: prefer-local
status: landed
created: 2026-08-29
---

# The edit you made survives the edit you were judged for

## Why

tCal merges a local calendar against a remote one and projects what the two could not settle as duplicate TOML keys. tCard does the same for a card, from the same plan, in the same week. They disagree about one thing, and neither author chose it.

The local side has to be the merge's right side here, because the right side is the only one ical-rs judges: an attendee may not move a meeting someone else organises (RFC 5546 section 3.2), and `right_speaks_for` is the only hook that says so. Until now the right side also lost every collision the projection could not render as a choice, so putting local there to have it judged was the same act as making it lose. tCard, whose vCards have no organiser and so nothing to judge, puts local on the left and keeps the local value. The result was that the same divergence, with the same shape and the same two tools, kept the remote value in one and the local value in the other.

ical-rs has split the two questions. `IcalMerge` now carries a `prefer: IcalMergeSide` alongside `left` and `right`: `left` still says whose untouched bytes the merged calendar is made of, and `prefer` says whose value survives where both sides wrote one. Authority stays where it was, on the replayed side, and a refusal does not depend on the preference. So tCal can keep local on the right, keep being judged, and stop paying a collision for it.

Which value should win, once it is a choice? The local one. The user is sitting in front of the document tCal opens, and the value they are shown as the merged state should be the one they wrote, not the one they are being asked to reconsider. It also makes the two tools one tool: a person merging a contact and a meeting in the same sync run should not have to remember which of the two quietly prefers the server.

## What

- Set `prefer: IcalMergeSide::Right` on the merge tCal runs, keeping local on the right so authority still reaches it.
- Restate, everywhere it is written down, that an unprojectable collision keeps the local value: the comment the document carries, the merge module header, the spec, and ARCHITECTURE.md.
- Leave the three settled reasons alone. A removal against an update still keeps the update whichever side it came from, a rule against an instance still keeps both, and a refusal for want of authority still refuses, none of them touched by the preference.
