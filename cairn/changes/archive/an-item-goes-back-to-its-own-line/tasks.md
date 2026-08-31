---
cairn: tasks
change: an-item-goes-back-to-its-own-line
---

- [x] Match each item back to the line that held it, and let an unheld item fill the room a line lost
- [x] Carry the original a folded line patches onto beside the line itself
- [x] Test: removing an item from a two-line `FREEBUSY` leaves the other line's `FBTYPE` alone
- [x] Test: removing an item from a two-line `CATEGORIES` leaves the other line's `LANGUAGE` alone
- [x] Test: renaming an item rewrites its own line rather than opening a second
- [x] Golden fixture asserting the round trip, with no `.lossy` marker
- [x] Fold the delta into cairn/spec/template.md and write the log entry
- [x] Note the fix in CHANGELOG.md
