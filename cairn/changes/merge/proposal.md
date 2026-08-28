---
cairn: change
id: merge
status: landed
created: 2026-08-28
---

# A collision has nowhere to be shown

## Why

ical-rs merges two divergent calendars against their common base and reports what collided. Something has to put that report in front of a person, and tCal is already the thing that puts an event in front of a person: the TOML projection exists so that a calendar can be read and edited by someone who does not want to think about folded lines, recurrence rules and UTC offsets.

A sync tool is the immediate caller. It merges in the background, resolves everything the two sides did not both touch, and is then holding one calendar, one base, one remote body and a short list of properties where the two sides disagree. It must not decide those itself and it must not open an editor from an unattended run, so it hands the three bodies to a program and takes back one. tCal is the natural program, and the piece it lacks is small: it already renders and parses the document, and the merge is a function it already links.

The interesting question is how an unresolved collision looks in TOML, because the obvious answers are all quietly lossy. Commenting the alternatives out leaves the property absent from the document, and absence is already how a user deletes one, so an ignored collision silently drops a property and looks exactly like an intended deletion. A separate block listing the candidates is unambiguous but needs a rule that something has to enforce.

TOML enforces one already: duplicate keys are a parse error. Writing the same key once per surviving side makes an unresolved document one that cannot be applied at all, with no vocabulary to invent and no rule to police, and resolution is deleting the lines you do not want.

Calendars add a second kind of report entry that a card does not have. A rule that moved and an instance that moved are both kept, deliberately, and said out loud because moving the rule may have moved the ground the instance stood on. A change refused because the editor does not speak for the organiser is likewise reported and already settled. Neither is a choice, and rendering either as one would ask the reader to decide something that is not theirs to decide.

## What

- A `merge` verb taking base, local and remote paths plus an output path, running the merge in process and projecting the result.
- An organiser calendar address as a flag, so the merge knows who the edited side speaks for and can refuse what is not theirs (RFC 5546 3.2). Where it is not given, nothing is claimed and nothing is refused on that ground.
- A collision rendered as duplicate keys, one live line per side, the ancestor above them as a comment so reverting stays possible and never accidental.
- The decided and informational entries rendered as header comments: a removal against an update, a rule against an instance, and a refusal for want of authority.
- A collision inside a nested component rendered as duplicate keys within the one table that projects it, never as a repeated array-of-tables block, which is legal TOML and would silently make a second alarm rather than an error.
- The duplicate-key parse error caught and reported as the property left undecided, rather than as a syntax error, reusing the reprompt loop the editor path already has.
