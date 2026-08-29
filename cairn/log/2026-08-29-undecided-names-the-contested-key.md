---
cairn: log
change: undecided-names-the-contested-key
date: 2026-08-29
---

# The refusal names a key the document does not write

The duplicate-key convention buys one thing: a document that will not apply, and a message telling the reader which property is still undecided. Two sides setting a different alarm trigger got `Property week is left undecided`. The document writes no `week`; it writes `trigger.week`, and the key in dispute is `trigger.min`. The one message the whole convention rests on sent the reader looking for a key that is not there, and named the wrong property while doing it.

## What landed

- **A contest is the lines the two sides spell differently.** `Choice` gained `keys`, every key the two sides write between them in the local side's order, and `contested`, the subset they disagree on. `Choice::render` walks the keys once: a contested one is written as the commented ancestor and one live line per side, an agreed one is written once, untagged, in the place it holds. A trigger differing only in minutes used to be fifteen lines, five of them commented and ten live; it is now eight, one of them commented and two live. The `# conflict, keep one side` header is chosen by the number of contested keys rather than the number of lines, so a value contested in two places still asks for a side.

- **A whole value is still whole.** The lines a reader has to decide between are still every line the sides disagree on, written once per side, so deleting one of a side's lines leaves the others duplicated and the document still refuses. The forcing that stops a spliced date, local time with a remote zone, is unchanged; what went is the duplication of lines where either copy is the other.

- **A collision the projection spells the same way is a comment.** Where every projected line agrees, `Sides::choice` returns nothing and the collision falls to the note the projection already writes for what it cannot address: the local value was kept, said in the header. Two sides setting a different `RSVP` on one attendee used to render eight lines of contest, four per side, every one of them identical to its opposite, and the reader was asked to choose between them.

- **The refusal names the dotted key.** `undecided` reads the key from the line the duplicate span sits on rather than from the span, which for a dotted key covers its last segment alone. `trigger.min`, not `min`.

## Verification

- The whole suite green: 61 lib, 15 merge forcing (2 still ignored, both blocked elsewhere), 4 projection laws, the fixture database, the doctest. `--no-default-features` and `--features merge` both build `no_std`, `clippy --all-features --all-targets` is clean, `cargo fmt` run.
- `the_refusal_names_a_key_the_document_writes` is no longer ignored.
- Four tests are new. A contested attendee now pins that the refusal names `status` rather than the table's first key, and that resolving it leaves one attendee for that address rather than two. Two sides changing a different key of one attendee are pinned not to collide at all. A zoned start moved to a different time in a different zone is pinned to contest both its lines, to refuse when only one remote line is deleted, and to yield the local time and the local zone together when the side goes out whole: the multi-line branch had no test before. An `RSVP` collision is pinned to a comment.

Capabilities moved: `merge` (MODIFIED: what a contest holds, and what the refusal names).
