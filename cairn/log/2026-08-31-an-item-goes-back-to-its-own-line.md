---
cairn: log
change: an-item-goes-back-to-its-own-line
date: 2026-08-31
---

# Removing one item relabelled the ones behind it

A repeated property kept its own line only while nobody edited the array. `spread` handed each line as many items as it held, counted off the front, so an item removed anywhere but the end slid every item behind it onto the line before, and the last line was dropped for want of items to fill it.

A free/busy report saying the morning was busy and the afternoon free:

    FREEBUSY;FBTYPE=BUSY:19980101T010000Z/19980101T020000Z,19980101T030000Z/PT1H
    FREEBUSY;FBTYPE=FREE:19980102T010000Z/PT2H

came back, after deleting the second busy period, as one line reporting the free afternoon busy:

    FREEBUSY;FBTYPE=BUSY:19980101T010000Z/19980101T020000Z,19980102T010000Z/PT2H

The same edit on two `CATEGORIES` lines relabelled the French category as English and dropped the French line. The law that covered repeated list properties folded an untouched projection, and untouched was the one case counting off the front got right.

The count was the wrong key. A line's parameters describe the items that line carried, so `spread` now matches each item back to the line whose value held it. An item no line held fills the room a line lost, which keeps a rename on its own line, and one past all that room still opens a line of its own. A line left with no item is removed.

Pairing a folded line with its original was positional too, which put a surviving line's value in the slot of the line before it. `content_lines` now carries the original beside each line it builds, so the pairing is stated rather than counted.

Three round-trip laws cover it, and the multi_valued fixture asserts the byte-exact round trip over two `CATEGORIES` lines, two `FREEBUSY` lines, and the `RESOURCES`, `EXDATE`, `RDATE` and quoted `MEMBER` list values the form does not model.

Capabilities moved: template.
