---
cairn: change
id: one-line-is-the-array
status: landed
created: 2026-08-31
---

# One line is the array, and leftovers share a line

## Why

The README's own front-page example says this:

```toml
categories = ["pimalaya", "cli"]
```
```ics
CATEGORIES:pimalaya,cli
```

It stopped being true. [a-repeated-property-keeps-its-line](../a-repeated-property-keeps-its-line/proposal.md) gave every item no line held a line of its own, so filling that field wrote two `CATEGORIES` properties rather than one. The regression is inside the tagged v0.1.0: the README was written against the behaviour before it.

Giving each leftover item its own line is defensible where several lines already exist and there is no answer to which one's parameters an added item should carry. It is not defensible where there is one line, or none: there is nothing to disambiguate, so the array is simply that line's items and an added one belongs to it.

## What

**At most one line is the array**, in the order the document wrote it. An added item joins the line and its parameters, which is what the README documents, and what someone editing a single-line property means.

**Whatever is left over shares one new line.** Where several lines do exist the ambiguity is real, so those items still get no parameters, but they get one line between them rather than one each.

Both twins carry the same rule: tCard's `ORG` already stayed one line by construction, and its `NICKNAME` had the same one-line-each behaviour.
