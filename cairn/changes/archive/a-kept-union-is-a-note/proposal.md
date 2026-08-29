---
cairn: change
id: a-kept-union-is-a-note
status: landed
created: 2026-08-29
---

# A list both sides edited is merged silently

## Why

Where both sides rewrite a multi-valued property, ical-rs merges them item by item. Neither side changed the property as far as the merge can see: one removed some items and added others, the other did the same, and the items merge as a set, so no conflict is recorded. The merged calendar carries every item either side kept, the document says nothing, and the reader is never told.

Merging items as a set is right, and deliberate. RFC 5545 gives the items of `CATEGORIES` no order, so two sides each adding a category should keep both, and asking a reader to choose between them would throw one away for no reason. This is not a defect to fix in ical-rs, and making it a contest here would be wrong.

What is wrong is the silence. The merged value is one neither side wrote, and the reader has no way to see that it was assembled rather than chosen. The item actions are already in the report, as `ValueItemAdded` and `ValueItemRemoved`; nothing reads them, so they reach neither a key nor the header.

tCard landed the same note today, as the change of the same id. This one ports it, so the two tools tell a reader the same thing.

## What

- Say in the header comment every list both sides edited, the way a collision the merge already settled is said, stating that the items of both were kept.
- Leave the merge itself alone: no contest, no duplicate keys, nothing for ical-rs to change.

## Where it cannot match tCard

tCard notes a `TYPE` parameter both sides edited as well, because vcard-rs reports parameter items apart, as `ParamItemAdded` and `ParamItemRemoved`. ical-rs reports a parameter whole, so two sides editing one collide and are contested like any other value. There is no second note to port, and adding one would say something untrue.

The note names the property the way every other tCal note does, by the block and key the projection writes it as (`event 1 / categories`), and ends in a full stop, since a tCal header comment is a wrapped sentence rather than tCard's bare clause.
