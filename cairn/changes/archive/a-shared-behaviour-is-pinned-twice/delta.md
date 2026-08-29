---
cairn: delta
change: a-shared-behaviour-is-pinned-twice
---

## ADDED Requirements

### Requirement: A read failure names the side it came from
Where one of a merge's three calendars does not parse, the refusal SHALL name the side it was given as, beside what the reader made of it.

A merge is the one verb reading more than one body, and its three paths are the user's. A refusal naming none of them says only that the merge failed, leaving the reader to open all three to find out which.

#### Scenario: An unreadable remote calendar
- GIVEN a merge whose remote calendar is not an iCalendar
- WHEN it is projected
- THEN it is refused, naming the remote side

### Requirement: A header note wraps at the document's column
A note written into the document header SHALL wrap at the same column the header itself uses, its `#` prefix included, and a continuation line SHALL be indented under the text of its bullet rather than under the bullet mark.

The header is prose a person reads before anything else, and a line running past the width the rest of the document keeps is the one part of the document that can leave the screen.

#### Scenario: A note longer than the column
- GIVEN a note whose text passes the wrapping column
- WHEN the document is projected
- THEN it is written over two comment lines, the second indented under the first line's text
