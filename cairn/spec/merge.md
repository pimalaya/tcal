---
cairn: spec
capability: merge
status: current
---

# Merge

Reconciling two divergent edits of one calendar, and putting what could not be reconciled to a person. The merge itself is ical-rs's, over a byte-faithful syntax tree; this crate owns the part that has a reader in it, which is how a collision is written so that it cannot be overlooked.

The vocabulary the document is written in is the projection every other verb uses, and folding an edited document back onto bytes is the same `apply` as anywhere else. Nothing here changes either: a merged document is an ordinary projected document with two additions, comments in its header and one key written more than once.

### Requirement: Merging is a verb over three files
`merge` SHALL take a base, a local and a remote calendar as paths, plus the path to write, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

The capability SHALL be built unconditionally. ical-rs is a plain dependency of every configuration, so gating the merge changes nothing about the crate set and a cargo feature has nothing left to buy.

Taking the three rather than a pre-merged body with markers is what keeps the document a calendar. Line markers are how a line-oriented merge shows an unresolved region, and an iCalendar is not lines: a marker in one would break every parser downstream, including this one. The merge is a pure function over bodies already at hand, so running it here rather than receiving its output costs nothing and invents no format.

The merged calendar SHALL be projected as the merge produced it, rather than written out and read back. Serialising between the reconciliation and the document is a second reading of the same body, and a byte the merge preserved that a second reading changed would reach the reader as the merge's own work.

The local side SHALL be the merge's left side, so the merged bytes are its own and a collision the merge does not settle holds its value, which the document then asks about rather than keeps quietly. tCard and neverest place it the same way, and so does a merge everywhere else: the side being merged into is the side that wins.

#### Scenario: The output is written only when the document is decided
- GIVEN a merge whose document still holds an undecided collision
- WHEN the editor exits
- THEN the output path is not written

#### Scenario: A collision keeps the local value
- GIVEN a local side changing a property the remote side also changed
- WHEN the merge is projected
- THEN the merged bytes carry the local value

### Requirement: A read failure names the side it came from
Where one of a merge's three calendars does not parse, the refusal SHALL name the side it was given as, beside what the reader made of it.

A merge is the one verb reading more than one body, and its three paths are the user's. A refusal naming none of them says only that the merge failed, leaving the reader to open all three to find out which.

#### Scenario: An unreadable remote calendar
- GIVEN a merge whose remote calendar is not an iCalendar
- WHEN it is projected
- THEN it is refused, naming the remote side

### Requirement: A collision is duplicate keys
A property both sides changed SHALL be written once per surviving side, each line naming its side, with the ancestor above them as a comment. Resolving SHALL be deleting the lines that are not wanted, or replacing all of them with a value of the user's own.

TOML forbids duplicate keys, so an undecided document does not parse and cannot be applied. The forcing is the format's rather than a rule of ours, which means there is nothing to enforce, nothing to name, and no way to save a decision that was never made. Commenting the alternatives out instead would leave the property absent, and absence is how a user deletes one, so an overlooked collision would drop a property and look deliberate.

The ancestor is a comment because keeping it is never the resolution to a collision: both sides moved away from it, and offering it as a live third line invites discarding two edits at once.

A field the projection writes as several lines (a date and its zone, a duration, a recurrence) SHALL contest every line its two sides spell differently, and SHALL write the lines they agree on once. Contesting the differing lines together is what keeps the value whole: half of one side and half of the other would be neither, and leaving one of them undeleted keeps the document unappliable, which is the forcing doing its work. A line both sides agree on is not a choice, since either copy is the other, and duplicating it would ask for a decision that decides nothing while burying the one that does not.

The refusal SHALL name the key as the document writes it, dotted path included, since a name the reader cannot find in the document is a riddle rather than help.

#### Scenario: An undecided document is refused
- GIVEN a merged document holding a collision as written
- WHEN it is applied
- THEN it is refused, naming the property left undecided rather than reporting a syntax error

#### Scenario: Deleting the other line decides it
- GIVEN the same document with one of the two lines removed
- WHEN it is applied
- THEN the calendar carries the surviving value

#### Scenario: The refusal names a key the document writes
- GIVEN a merged document contesting one part of a value the projection writes as several lines
- WHEN it is applied
- THEN the refusal names that part under the dotted key the document writes it as

#### Scenario: A multi-line value is contested whole
- GIVEN two sides moving a start to a different time in a different zone
- WHEN the document is projected
- THEN both the date and its zone are written once per side, and deleting only one line of a side still refuses

### Requirement: Only a genuine choice is rendered as one
A report entry the merge already settled SHALL be a header comment, not duplicate keys. Two settle themselves: a removal against an update, where the update wins whichever side it came from; and a rule change against an instance change, where both survive and the reader is being warned that the rule may have moved the ground the instance stood on.

Rendering either of them as a choice would ask a reader to decide something already decided, and in one of the two cases a candidate could not be written as a line at all.

A collision on something the projection does not model is likewise a comment, naming what changed and saying that the local value was kept: there is no key to write it twice under, and inventing one would put a property in the document that applying it could not carry back. A collision the projection spells the same way on both sides is the same case: the difference sits in something it never shows, so there is nothing to put to a reader.

#### Scenario: A warned pair is not a choice
- GIVEN a merge where one side changed the recurrence rule and the other moved an overriding instance
- WHEN the document is projected
- THEN both changes are written once and the pair is said in a comment

#### Scenario: An unprojectable collision keeps the local value
- GIVEN a merge where both sides changed a part of a property the projection does not model
- WHEN the document is projected
- THEN the local value is in the merged bytes and the comment says so

#### Scenario: A collision the projection cannot tell apart is a comment
- GIVEN two sides setting a different unshown parameter on the same attendee
- WHEN the document is projected
- THEN the attendee is written once and the collision is said in a comment

### Requirement: A union is said in the header
Where both sides edited the items of one multi-valued property, the document SHALL say so in its header comment, stating that the items of both were kept. It SHALL NOT contest them.

The items of such a value merge as a set, RFC 5545 giving them no order, so both sides' additions and removals all apply and nothing collides. That is the right outcome: two sides each adding a category should keep both, and putting them to a reader would throw one away for no reason. The silence is what is wrong, since the merged value is then one neither side wrote and nobody was told it was assembled.

#### Scenario: Both sides rewrite a list
- GIVEN a base holding `CATEGORIES:a,b`, a local holding `CATEGORIES:c,d` and a remote holding `CATEGORIES:e,f`
- WHEN they are merged
- THEN the calendar holds all four, the header says the items of both were kept, and the document applies as it stands

### Requirement: The document holds every conflict it announces
The preamble SHALL announce as many conflicts as the document writes contests below it. A collision the document found no line to contest SHALL become a header comment saying the local value was kept, the same comment an unaddressable collision becomes, rather than being dropped.

The announcement is the reader's whole instruction: it says the document cannot be applied until every conflict is decided. A document announcing one and showing none sends them looking for a key that is not there, then applies without a word and takes one side. Counting what was written rather than what was reported makes the two numbers one number, and the fallback is what keeps that from costing a report.

#### Scenario: A conflict with nowhere to go is still said
- GIVEN a collision the document has no line to contest
- WHEN the document is projected
- THEN the preamble announces no conflict, and the collision is said in a comment

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

### Requirement: Deciding a collision keeps what the document never showed
Folding an edited document back SHALL keep every parameter of a modelled property that the projection does not show, in the place it held on the line. A parameter the projection does show SHALL be the document's: taken from the edited value, and dropped where the document cleared it.

The projection is an editing affordance rather than an interchange format, so what it does not show it does not own. A line is patched rather than rebuilt: the value and the shown parameters come from the document, the rest of the line is the calendar's own. Rebuilding dropped `RSVP`, which decides whether a scheduling client asks an attendee at all, and `SENT-BY`, which is who may speak for the organiser, so settling a conflict by hand quietly undid what those parameters carry.

A property's lines are paired with the document's by position, which is how the projection shows them in the first place.

#### Scenario: An unshown parameter survives a decision
- GIVEN a merged document whose attendee carries a parameter the form has no key for
- WHEN the document is applied
- THEN the attendee line still carries it

#### Scenario: A shown parameter is the document's to clear
- GIVEN a merged document with an attendee's status emptied
- WHEN the document is applied
- THEN the status parameter is gone and the rest of the line is unchanged

### Requirement: A header note wraps at the document's column
A note written into the document header SHALL wrap at the same column the header itself uses, its `#` prefix included, and a continuation line SHALL be indented under the text of its bullet rather than under the bullet mark.

The header is prose a person reads before anything else, and a line running past the width the rest of the document keeps is the one part of the document that can leave the screen.

#### Scenario: A note longer than the column
- GIVEN a note whose text passes the wrapping column
- WHEN the document is projected
- THEN it is written over two comment lines, the second indented under the first line's text
