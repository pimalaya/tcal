---
cairn: log
change: a-repeated-property-keeps-its-line
date: 2026-08-31
---

# A component's second list property was dropped on the way back

A list field projected the items of every line it held into one array and folded that array back into one line. A component carrying `CATEGORIES;LANGUAGE=en:a,b` beside `CATEGORIES;LANGUAGE=fr:c` came back as a single `CATEGORIES:a,b,c`, the second line gone and its parameter with it.

It failed the law every other one rests on. The loss needed no edit: folding an untouched projection changed the calendar, which is the one thing the projection promises not to do. Nothing caught it because no fixture held two properties of one list name, and the property-based generator wrote at most one `CATEGORIES` line.

`spread` now walks the array back over the lines it came from, each keeping as many items as it held, an item past the last of them opening its own line. The patching that follows pairs the lines with their originals by position, as it already did, so each keeps the parameters the form never showed. tCard answered this first and its `spread` is the same shape; the two now agree.

A round-trip law covers it: two properties of one repeatable name, each with a different `LANGUAGE`, come back as two, byte for byte, and a second pass changes nothing.

Capabilities moved: template.
