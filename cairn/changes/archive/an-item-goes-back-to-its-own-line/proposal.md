---
cairn: change
id: an-item-goes-back-to-its-own-line
status: landed
created: 2026-08-31
---

# Removing one item relabels the ones behind it

## Why

A repeated property keeps its own line, but only for as long as nobody edits the array. `spread` hands each line as many items as it held, counted off the front of the array, so an item removed anywhere but the end slides every item behind it onto the line before, and the last line is dropped for want of items to fill it.

A calendar reporting a busy morning and a free afternoon:

    FREEBUSY;FBTYPE=BUSY:19980101T010000Z/19980101T020000Z,19980101T030000Z/PT1H
    FREEBUSY;FBTYPE=FREE:19980102T010000Z/PT2H

Deleting the second busy period from the array leaves:

    FREEBUSY;FBTYPE=BUSY:19980101T010000Z/19980101T020000Z,19980102T010000Z/PT2H

The free afternoon is now reported busy. The same edit on two `CATEGORIES` lines relabels a French category as English and drops the French line:

    CATEGORIES;LANGUAGE=en:work,travel      CATEGORIES;LANGUAGE=en:work,travail
    CATEGORIES;LANGUAGE=fr:travail       ->

Nothing catches it because the law that covers repeated list properties folds an untouched projection, and untouched is the one case counting off the front gets right.

The count is the wrong key. A line's parameters describe the items that line carried, so an item belongs to the line whose value it came out of, not to a position in a flattened array.

## What

- Give each item back to the line it came out of, matched on the value the projection showed.
- Let an item no line held fill the room a line lost, so renaming an item still rewrites the line it was on rather than opening a new one.
- Pair a folded line with the original it patches onto explicitly, rather than by its position among the lines, so a line that lost every item is deleted instead of donating its parameters to the next.
