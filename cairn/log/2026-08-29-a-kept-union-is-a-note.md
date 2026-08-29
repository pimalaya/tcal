---
cairn: log
change: a-kept-union-is-a-note
landed: 2026-08-29
---

# A list both sides edited is merged silently

Where both sides rewrote a multi-valued property, the merged calendar came out carrying every item either side kept, and the document said nothing. `CATEGORIES:a,b` against `c,d` and `e,f` merged to `e,f,c,d`, a value neither side wrote, in an order neither side chose.

## What landed

- **The union is a header note.** `Sides::note_unions` reads the item actions of both sides out of the report and, where they name the same list on the same property, says so: `event 1 / categories: both sides changed its list; the items of both were kept.` It joins the notes the header already carries for what the merge settled on its own, so the reader meets it where they already look.

- **The merge itself is untouched.** The items still merge as a set, no contest is written, and the document still applies as it stands. There is nothing to choose: RFC 5545 gives the items of `CATEGORIES` no order, so both sides' additions and removals all apply, and putting them to a reader would throw one of two categories away for no reason.

- **The finding was wrong about the remedy** and is corrected: the union is not a defect and neither ical-rs nor this crate's contest needed changing. What was missing is the note, which is now there.

## Ported from tCard, with one difference

tCard landed the same note today under the same change id, and this is its port. tCard notes a `TYPE` parameter both sides edited as well, because vcard-rs reports parameter items apart. ical-rs reports a parameter whole, so two sides editing one collide and are contested like any other value: there is no second note to port. The note also names the property the tCal way, by the block and key the projection writes it as, and ends in a full stop, tCal header comments being wrapped sentences rather than bare clauses.

## Verification

The reproduction is no longer ignored: `a_list_union_is_said_in_the_header` asserts the merged value, the header line, that no contest is written, and that the document applies. The merge forcing suite is 17 tests with none ignored, and it is now the whole suite: nothing is left ignored anywhere. All three feature configurations build, clippy is clean, `cargo fmt` run.

Capabilities moved: merge (ADDED: A union is said in the header).
