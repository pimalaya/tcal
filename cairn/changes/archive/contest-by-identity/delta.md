---
cairn: delta
change: contest-by-identity
---

## ADDED Requirements

### Requirement: The document holds every conflict it announces
The preamble SHALL announce as many conflicts as the document writes contests below it. A collision the document found no line to contest SHALL become a header comment saying the local value was kept, the same comment an unaddressable collision becomes, rather than being dropped.

The announcement is the reader's whole instruction: it says the document cannot be applied until every conflict is decided. A document announcing one and showing none sends them looking for a key that is not there, then applies without a word and takes one side. Counting what was written rather than what was reported makes the two numbers one number, and the fallback is what keeps that from costing a report.

#### Scenario: A conflict with nowhere to go is still said
- GIVEN a collision the document has no line to contest
- WHEN the document is projected
- THEN the preamble announces no conflict, and the collision is said in a comment

## MODIFIED Requirements

### Requirement: A nested collision stays inside its table
A collision inside a nested component SHALL be rendered as duplicate keys within the single table that projects it, and SHALL NOT be rendered as a repeated array-of-tables block. Repeating such a header is valid TOML and would produce a second alarm or attendee rather than a parse error, so the forcing that makes the whole convention safe would silently vanish exactly where the structure is deepest.

The addressing is not this crate's to derive. The merge report names the component a property belongs to, by `UID` and `RECURRENCE-ID` where there is one, and its position among its siblings where there is not, so a collision addresses one projected key however deep it sits.

An attendee is the one property the projection writes as a table rather than a key, so a contested attendee is contested key by key inside the one table it wrote, for the same reason. Which attendee SHALL be resolved by the identity the report carries, the calendar address, in each calendar the projection reads: the merged one it writes the contest into, and the three the sides' lines come from. The position the report also carries is counted in the side that wrote the action, so it names a different property in every calendar a removal shifted, and it SHALL be used only where iCalendar gives the property no identity.

#### Scenario: One alarm, one contested key
- GIVEN two sides setting a different trigger on the same alarm
- WHEN the document is projected
- THEN one alarm table is written, its trigger contested and its other keys written once

#### Scenario: A removal beside a contested attendee
- GIVEN a local side removing one attendee and answering as another, and a remote side answering differently as that other
- WHEN the document is projected
- THEN the surviving attendee's answer is contested in the one table it wrote
