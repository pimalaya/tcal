---
cairn: change
id: a-repeated-property-keeps-its-line
status: landed
created: 2026-08-31
---

# A component's second list property is dropped on the way back

## Why

A list field projects the items of every line it holds into one array, and folded one array back into one line. A component carrying `CATEGORIES;LANGUAGE=en:a,b` beside `CATEGORIES;LANGUAGE=fr:c` therefore came back as a single `CATEGORIES:a,b,c`: the second line was gone, and with it the parameters the form never showed.

The loss is silent and it is the projection's own doing. It happens on an untouched document, so it fails the law every other one rests on, folding an untouched projection changing nothing. No fixture held two properties of one list name, so nothing caught it.

tCard answered this first, spreading the items back over the lines they came from.

## What

- Spread a list field's items over its own lines on the way back, each line keeping as many items as it held and a surplus opening its own line.
- Add the round-trip law for two properties of one repeatable name.
