---
cairn: log
change: one-line-is-the-array
landed: 2026-08-31
---

# One line is the array, and leftovers share a line

The README's front-page example, `categories = ["pimalaya", "cli"]` for `CATEGORIES:pimalaya,cli`, had stopped being true: the field wrote two `CATEGORIES` properties. [an-item-goes-back-to-its-own-line](../changes/archive/an-item-goes-back-to-its-own-line/proposal.md) gave every item no line held a line of its own, which is right where several lines already exist and wrong where there are none. `git merge-base --is-ancestor` puts the regression inside the tagged v0.1.0.

**At most one line is now the array** (template/model.rs), in document order. There is nothing to disambiguate, so the ownership matching is skipped entirely and the items are that line's, an added one joining its parameters. That is both the README's spelling and what someone editing a single-line property means.

**Leftover items share one new line** where several lines do exist. The ambiguity is real there, so they still carry no parameters, but they get one line between them rather than one each.

The same two rules landed in tCard, where `NICKNAME` behaved identically and `ORG` was already one line by construction.

Verified: 54 unit tests, 17 merge, 13 projection laws and the 11 fixtures green, two of them new over the README's example and the shared new line. Driven against the built binary for four cases: an added item joining a lone line with its `LANGUAGE`, two added items sharing one bare line beside two parameterised ones, a removal leaving both lines alone, and a rename staying on its own line.

Spec updated: `template` (ADDED: "One line leaves nothing to disambiguate"; MODIFIED: "A repeated property keeps its own line").
