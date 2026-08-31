---
cairn: delta
change: prefer-local
---

## MODIFIED Requirements

### Requirement: Merging is a verb over three files
`merge` SHALL take a base, a local and a remote calendar as paths, plus the path to write, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

It SHALL take the calendar address the edited side speaks for as an option, passed through to the merge, and SHALL claim nothing where it is not given.

Taking the three rather than a pre-merged body with markers is what keeps the document a calendar. Line markers are how a line-oriented merge shows an unresolved region, and an iCalendar is not lines: a marker in one would break every parser downstream, including this one. The merge is a pure function over bodies already at hand, so running it here rather than receiving its output costs nothing and invents no format.

The local side is the merge's right side: the edited one, whose actions are replayed onto the remote side's bytes and on whose behalf authority is claimed. It SHALL also be the preferred side, so a collision the merge does not settle holds the local value in the merged bytes, which the document then asks about rather than keeps quietly. The two are separate statements: being replayed is what makes the local side judgeable, and being preferred is what stops that judgement costing it every collision. This is what tCard does with local on the left, where a card has no organiser and nothing to judge.

#### Scenario: The output is written only when the document is decided
- GIVEN a merge whose document still holds an undecided collision
- WHEN the editor exits
- THEN the output path is not written

#### Scenario: Being judged does not cost the collision
- GIVEN a local side speaking for an attendee, changing a property both sides changed
- WHEN the merge is projected
- THEN the merged bytes carry the local value, and a change the attendee has no authority over is still refused

### Requirement: Only a genuine choice is rendered as one
A report entry the merge already settled SHALL be a header comment, not duplicate keys. Three settle themselves: a removal against an update, where the update wins whichever side it came from; a rule change against an instance change, where both survive and the reader is being warned that the rule may have moved the ground the instance stood on; and a change refused because the edited side does not speak for the organiser, which is not the reader's to reverse.

Rendering any of them as a choice would ask a reader to decide something already decided, and in two of the three cases one of the candidates could not be written as a line at all.

A collision on something the projection does not model is likewise a comment, naming what changed and saying that the local value was kept: there is no key to write it twice under, and inventing one would put a property in the document that applying it could not carry back.

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
