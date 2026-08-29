---
cairn: delta
change: undecided-names-the-contested-key
---

## MODIFIED Requirements

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
A report entry the merge already settled SHALL be a header comment, not duplicate keys. Three settle themselves: a removal against an update, where the update wins whichever side it came from; a rule change against an instance change, where both survive and the reader is being warned that the rule may have moved the ground the instance stood on; and a change refused because the edited side does not speak for the organiser, which is not the reader's to reverse.

Rendering any of them as a choice would ask a reader to decide something already decided, and in two of the three cases one of the candidates could not be written as a line at all.

A collision on something the projection does not model is likewise a comment, naming what changed and saying that the local value was kept: there is no key to write it twice under, and inventing one would put a property in the document that applying it could not carry back. A collision the projection spells the same way on both sides is the same case: the difference sits in something it never shows, so there is nothing to put to a reader.

#### Scenario: A warned pair is not a choice
- GIVEN a merge where one side changed the recurrence rule and the other moved an overriding instance
- WHEN the document is projected
- THEN both changes are written once and the pair is said in a comment

#### Scenario: A refusal is reported, not offered
- GIVEN a merge where the edited side does not speak for the organiser and changed the start
- WHEN the document is projected
- THEN the start is the organiser's, and the refusal is said in a comment

#### Scenario: An unprojectable collision keeps the local value
- GIVEN a merge where both sides changed a part of a property the projection does not model
- WHEN the document is projected
- THEN the local value is in the merged bytes and the comment says so

#### Scenario: A collision the projection cannot tell apart is a comment
- GIVEN two sides setting a different unshown parameter on the same attendee
- WHEN the document is projected
- THEN the attendee is written once and the collision is said in a comment
