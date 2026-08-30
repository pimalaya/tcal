---
cairn: log
change: local-on-the-left
date: 2026-08-30
---

# The local calendar moved to the merge's left

tcal used to put local on the right and ask for the right side to be preferred. The two together kept the local value winning a collision, but they were a workaround: authority could only be claimed for the right side, so the side that had to be judged was the side local had to occupy.

ical-rs dropped that field, so the workaround has nothing to work around. Local is the left side now, which makes the merged bytes its own and makes it win a collision by default, with no preference stated. That is where tcard and neverest already put it.

Reading a conflict inverted with it. The report names the right side's action and carries the left side's in the reason, so what used to be the local action is now the remote one and the reason's payload is the local one; the conflict reader takes them that way round rather than by their old names.

`merge --speaks-for` is gone with the refusal it bought. One visible consequence beyond that: where both sides add to the same list, the local items now lead, the union being written onto the local side's bytes.

Capabilities moved: merge.
