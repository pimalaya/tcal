---
cairn: change
id: undecided-names-the-contested-key
status: landed
created: 2026-08-29
---

# The refusal names a key the document does not write

## Why

The duplicate-key convention rests on one sentence: when the document will not apply, the reader is told which property is still undecided. Two sides setting a different alarm trigger produce this refusal:

    Property week is left undecided: keep one of its lines and delete the others

The document writes no `week` key. It writes `trigger.week`, and the property that actually differs is `trigger.min`. A reader searching for what they were told to find does not find it, and the one thing the convention has to get right is the one thing it gets wrong.

Two causes, both in the merge module:

- `undecided` takes the span toml_edit reports for a duplicate key, which for a dotted key covers only its last segment, so `trigger.week` is reported as `week`.
- `Choice::render` writes every line of a multi-line value once per side, the lines both sides spell identically included, so the first duplicate key in the document is `trigger.week` rather than the one in dispute. The attendee case has the same shape without the dots: the refusal names `display-name`, the first key of the contested table, where the key that differs is `status`.

The second cause is worse than a wrong name. Where the sides differ only in parameters the projection never shows (an attendee's `RSVP`), every line of the contest is identical on both sides, and the reader is asked to choose between two spellings of the same thing.

## What

Contest the lines the two sides spell differently, and write the rest once. A line both sides agree on is not a choice: keeping either copy keeps the same value, so duplicating it costs the reader a decision that decides nothing and hides the one that does not.

Where every projected line agrees, there is nothing to put to a reader at all, and the collision becomes a header comment saying the local value was kept, which is what an unprojectable collision already does.

Name the whole dotted key in the refusal, read from the line the duplicate sits on rather than from the span alone.
