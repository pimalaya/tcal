---
cairn: log
change: keep-unshown-parameters
date: 2026-08-29
---

# A modelled property loses every parameter the form does not show

The projection promises that what it does not model is kept verbatim, and a property outside the vocabulary is. A parameter of a property inside it was not: the form has keys for `CN`, `ROLE` and `PARTSTAT` on an attendee and for a date's zone, and folding the document back rebuilt the line out of those keys alone. `RSVP`, `CUTYPE`, `SENT-BY`, `ALTREP` and `LANGUAGE` went every time the line was written, which is every time anyone edits the event or settles a conflict in it. `SENT-BY` says who may speak for the organiser, the authority the merge refuses changes over; the editor was undoing by hand what the merge protects.

## What landed

- **A patch module.** template/patch.rs takes a content line apart on the colon that ends its parameters, a colon inside a quoted parameter value (`SENT-BY="mailto:s@x"`) not counting, and splits the prefix on the semicolons outside quotes. `rewritten` walks the original's parameters in order: one the projection shows is replaced by the document's spelling or dropped where the document cleared it, one it does not show is kept in the place it stood, and a parameter only the document writes is appended. tCard has the same module under the same name, from the same finding.

- **Each field names the parameters it writes.** `Field::params` answers `CN`/`ROLE`/`PARTSTAT` for an attendee, `TZID`/`VALUE` for a date, and nothing for everything else, so a `VALUE=PERIOD` on a free-busy line or a `VALUE=DATE-TIME` on an absolute trigger is now the line's own and survives.

- **`content_lines` folds onto the lines it came from.** It takes the component's own lines for the property, paired by position, which is the order the projection shows them in and the pairing the rest of the crate already uses. `block_has_content`, which only asks whether a block holds anything, passes none.

## Verification

- The whole suite green: 66 lib, 15 merge forcing, 5 projection laws, the fixture database, the doctest. `--no-default-features` and `--features merge` both build and test `no_std`, `clippy --all-features --all-targets` is clean, `cargo fmt` run.
- `a_modelled_property_keeps_its_unshown_parameters` is no longer ignored.
- The projection generator now writes a `LANGUAGE` on the summary and an `RSVP` on attendees, so the three standing laws (an untouched fold changes nothing, a second pass changes nothing, an unmodelled property survives) are held to unshown parameters as well rather than to a targeted case alone.
- One existing expectation moved rather than breaking: an absolute trigger, which falls back to a raw key, now comes back with the `VALUE=DATE-TIME` that types it instead of losing it.

Capabilities moved: `merge` (ADDED: what folding a decided document keeps).
