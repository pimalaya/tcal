---
cairn: spec
capability: merge
status: current
---

# Merge

Reconciling two divergent edits of one calendar, and putting what could not be reconciled to a person. The merge itself is ical-rs's, over a byte-faithful syntax tree; this crate owns the part that has a reader in it, which is how a collision is written so that it cannot be overlooked.

The whole capability is opt-in, behind a `merge` cargo feature that the `cli` feature pulls in. Linking a second iCalendar library is the one thing this crate does that a consumer wanting only the projection has no use for, so it is the one thing that is not paid for by default.

The vocabulary the document is written in is the projection every other verb uses, and folding an edited document back onto bytes is the same `apply` as anywhere else. Nothing here changes either: a merged document is an ordinary projected document with two additions, comments in its header and one key written more than once.

### Requirement: Merging is a verb over three files
`merge` SHALL take a base, a local and a remote calendar as paths, plus the path to write, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

It SHALL take the calendar address the edited side speaks for as an option, passed through to the merge, and SHALL claim nothing where it is not given.

Taking the three rather than a pre-merged body with markers is what keeps the document a calendar. Line markers are how a line-oriented merge shows an unresolved region, and an iCalendar is not lines: a marker in one would break every parser downstream, including this one. The merge is a pure function over bodies already at hand, so running it here rather than receiving its output costs nothing and invents no format.

The local side is the merge's right side: the edited one, whose actions are replayed onto the remote side's bytes and on whose behalf authority is claimed. A collision the merge does not settle therefore holds the remote value in the merged bytes, which the document then asks about rather than keeps quietly.

#### Scenario: The output is written only when the document is decided
- GIVEN a merge whose document still holds an undecided collision
- WHEN the editor exits
- THEN the output path is not written

### Requirement: A collision is duplicate keys
A property both sides changed SHALL be written once per surviving side, each line naming its side, with the ancestor above them as a comment. Resolving SHALL be deleting the lines that are not wanted, or replacing all of them with a value of the user's own.

TOML forbids duplicate keys, so an undecided document does not parse and cannot be applied. The forcing is the format's rather than a rule of ours, which means there is nothing to enforce, nothing to name, and no way to save a decision that was never made. Commenting the alternatives out instead would leave the property absent, and absence is how a user deletes one, so an overlooked collision would drop a property and look deliberate.

The ancestor is a comment because keeping it is never the resolution to a collision: both sides moved away from it, and offering it as a live third line invites discarding two edits at once.

A field the projection writes as several lines (a date and its zone, a duration, a recurrence) is contested whole, since its lines are one value and half of each side would be neither.

#### Scenario: An undecided document is refused
- GIVEN a merged document holding a collision as written
- WHEN it is applied
- THEN it is refused, naming the property left undecided rather than reporting a syntax error

#### Scenario: Deleting the other line decides it
- GIVEN the same document with one of the two lines removed
- WHEN it is applied
- THEN the calendar carries the surviving value

### Requirement: Only a genuine choice is rendered as one
A report entry the merge already settled SHALL be a header comment, not duplicate keys. Three settle themselves: a removal against an update, where the update wins whichever side it came from; a rule change against an instance change, where both survive and the reader is being warned that the rule may have moved the ground the instance stood on; and a change refused because the edited side does not speak for the organiser, which is not the reader's to reverse.

Rendering any of them as a choice would ask a reader to decide something already decided, and in two of the three cases one of the candidates could not be written as a line at all.

A collision on something the projection does not model is likewise a comment, naming what changed and which value was kept: there is no key to write it twice under, and inventing one would put a property in the document that applying it could not carry back.

#### Scenario: A warned pair is not a choice
- GIVEN a merge where one side changed the recurrence rule and the other moved an overriding instance
- WHEN the document is projected
- THEN both changes are written once and the pair is said in a comment

#### Scenario: A refusal is reported, not offered
- GIVEN a merge where the edited side does not speak for the organiser and changed the start
- WHEN the document is projected
- THEN the start is the organiser's, and the refusal is said in a comment

### Requirement: A nested collision stays inside its table
A collision inside a nested component SHALL be rendered as duplicate keys within the single table that projects it, and SHALL NOT be rendered as a repeated array-of-tables block. Repeating such a header is valid TOML and would produce a second alarm or attendee rather than a parse error, so the forcing that makes the whole convention safe would silently vanish exactly where the structure is deepest.

The addressing is not this crate's to derive. The merge report names the component a property belongs to, by `UID` and `RECURRENCE-ID` where there is one, and its position among its siblings where there is not, so a collision addresses one projected key however deep it sits.

An attendee is the one property the projection writes as a table rather than a key, so a contested attendee is contested key by key inside the one table it wrote, for the same reason.

#### Scenario: One alarm, one contested key
- GIVEN two sides setting a different trigger on the same alarm
- WHEN the document is projected
- THEN one alarm table is written, its trigger contested and its other keys written once
