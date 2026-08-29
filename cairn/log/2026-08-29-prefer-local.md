---
cairn: log
change: prefer-local
date: 2026-08-29
---

# The edit you made survives the edit you were judged for

tCal put the local calendar on the merge's right side because that is the only side ical-rs judges: an attendee may not move a meeting someone else organises (RFC 5546 section 3.2), and `right_speaks_for` is the hook that says so. Until today the right side also lost every collision, so putting local there to have it judged was the same act as making it lose. tCard, whose cards have no organiser and so nothing to judge, put local on the left and kept the local value. The same divergence, in two tools built as a pair from one plan, resolved two different ways, and neither author chose that.

ical-rs split the two questions. `IcalMerge` now carries `prefer: IcalMergeSide` beside `left` and `right`: `left` still says whose untouched bytes the merged calendar is built from, `prefer` says whose value survives where both sides wrote one, and authority stays on the replayed side either way. tCal keeps local on the right and sets `prefer: IcalMergeSide::Right`, so the local edit is both judged and kept.

## What landed

- **One field on the merge.** `IcalMerge { base, left: &remote, right: &local, right_speaks_for, prefer: IcalMergeSide::Right }`. Nothing else in `Merge::project` moved: the report is read the same way, the conflicts are addressed onto projected keys the same way, and the merged bytes are still the source the projection reads and `apply` patches.

- **The comment says which value it kept.** A collision on something the projection does not model is written as a header comment, since there is no key to write it twice under, and that comment now reads "the local value was kept". It is the only string in the module that named a side as the winner. The three settled reasons are untouched, and deliberately so: a removal against an update still keeps the update whichever side it came from, a rule against an instance still keeps both, and a refusal for want of authority still refuses. The preference reaches none of them.

- **The prose caught up.** The merge module header, the spec's `Merging is a verb over three files`, the FAQ answer in README.md, and the merge bridge section of ARCHITECTURE.md all said a collision holds the remote value. They now say the local one, and say why the two facts are separate: being replayed is what makes the local side judgeable, being preferred is what stops that judgement costing it every collision.

## What did not need to change, and why

The parent expectation was that the code locating a contested line in the projection would be keyed on the value the merge kept, as tCard's is, and would have to start looking for the local value. tCal's is not. `Choice::contests` matches a projected line by its TOML key, and the block a choice belongs to is addressed structurally, by walking the merge's component path (`UID`, then `RECURRENCE-ID`, then position among same-named siblings) and, for an attendee, by the index the merge report carries. None of that reads a value, so none of it moved. tCard needs the value because it picks among repeated typed blocks (`[[phone]]`, `[[address]]`) that its report does not index for it.

## Verification

- 63 tests green (61 lib, 1 fixture, 1 doctest), `cargo build --no-default-features` and `--features merge` both `no_std`, `cargo clippy --all-features --all-targets` clean, `cargo fmt`.
- Two tests are new. One takes a property outside the vocabulary that both sides changed, which the projection does not model, and asserts the merged bytes carry the local value and the comment says so. The other gives that same case an attendee who also tried to move the start: the start stays the organiser's and is reported refused, while the property that is hers to set keeps her value, which is the case the whole change exists for.
- One existing test gained an assertion rather than losing one: the contested-summary case now pins that the merged bytes carry the local value, which nothing had pinned before. No existing expectation was inverted, because no existing test asserted which side won a collision: the projectable ones replace the merged line with the choice block, so the merged value never reached an assertion.

Capabilities moved: `merge` (MODIFIED: which side wins a collision, and the comment that says so).
