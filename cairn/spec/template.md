---
cairn: spec
capability: template
status: current
---

# Template

Projecting a calendar as an ergonomic TOML form and folding the edited form back onto it. The document is an editing affordance rather than an interchange format, so what survives the round trip is what the rules here are about. How a body is read belongs to reading, and what a merge document says belongs to merge.

### Requirement: The projection is a sibling module, not an aggregator
The projection SHALL live in src/template.rs beside its src/template/ folder, rather than in src/template/mod.rs, because it carries the engine itself and not only the declarations of the modules under it.

The mod.rs choice is content-based. A folder whose mod.rs holds nothing but module declarations and re-exports keeps it; a module carrying code of its own is a sibling file next to the folder, so a reader can tell the two apart by the file name alone.

#### Scenario: Where the projection lives
- GIVEN the projection engine and the leaf modules it declares
- WHEN the source tree is read
- THEN the engine is src/template.rs and the leaf modules are files under src/template/

### Requirement: A repeated property keeps its own line
Where a component holds more than one property of a list field's name, folding a document back SHALL give each item to the line whose value it came out of, matched on the value the projection showed. An item no line held SHALL fill the room a line lost, in the order the document writes them, and whatever is left over SHALL share one new line between them. A line left with no item SHALL be removed rather than take the items of another.

The form shows the items of every such line as one array, which is what makes the field editable at all. A line's parameters describe the items that line carried, so counting items off the front of the array instead hands them to whichever line has room: removing one item relabels every item behind it and drops the last line, on an edit that named neither.

#### Scenario: Two properties of one list name
- GIVEN a component holding two `CATEGORIES` properties, each carrying a different `LANGUAGE`
- WHEN an untouched projection is folded back
- THEN the component still holds two, each with the items and the parameter it had

#### Scenario: An item removed from the first of two lines
- GIVEN a component holding `FREEBUSY;FBTYPE=BUSY` with two periods beside `FREEBUSY;FBTYPE=FREE` with one
- WHEN the document is folded back with one busy period removed
- THEN the busy line carries the remaining busy period and the free line carries its own, unchanged

#### Scenario: An item renamed in place
- GIVEN a component holding one `CATEGORIES` line of two items
- WHEN the document is folded back with one item replaced by a value no line held
- THEN the line still carries two items and no second line is written

### Requirement: A component-dependent vocabulary is hinted per component
Where RFC 5545 closes a vocabulary differently per component, the hint a block writes SHALL be the one its own component defines. That covers `STATUS` (3.8.1.11) and an attendee's `PARTSTAT` (3.2.12): a to-do also accepts `COMPLETED` and `IN-PROCESS`, and a journal accepts neither those nor `TENTATIVE` and `DELEGATED`.

A blank form is the documentation of what a field takes, so a hint listing another component's values is worse than none: it names values the component does not define and hides the ones it does.

#### Scenario: The participation statuses of a to-do and a journal
- GIVEN a blank projection of the whole calendar
- WHEN the attendee blocks are read
- THEN `[[todo.attendee]]` offers `completed` and `in-process`, `[[journal.attendee]]` offers neither those nor `tentative` and `delegated`, and `[[event.attendee]]` offers the event's five

### Requirement: One line leaves nothing to disambiguate
A list field holding at most one line SHALL take the array as that line's items, in the order the document wrote them. There is no second line to attribute an item to, so an added item joins the line and the parameters it carries.

That is the spelling the README documents, `categories = ["pimalaya", "cli"]` for `CATEGORIES:pimalaya,cli`, and it is what someone editing a single-line property means by adding to it.

#### Scenario: A second item added to a lone line
- GIVEN a component holding one `CATEGORIES` property, or none
- WHEN the document is folded back with a second item in the array
- THEN the component holds one `CATEGORIES` property carrying both items

### Requirement: The fold back is a verb of its own
`apply` SHALL take an edited TOML document and the calendar it was projected from, fold the one onto the other, and write the resulting iCalendar. It SHALL spawn nothing.

The projection is a round trip, and only its outward half was a verb: a form edited by a script, a filter or a graphical app had no way back, though the library exposes the fold back and `edit` uses it. Who filled the form is none of tCal's business.

The document SHALL be a path or `-` for stdin, and both inputs SHALL NOT be stdin at once. `apply` SHALL take the same component-type flags as `template`, since a type the form does not show is one the fold back leaves alone, and the same template has to be reconstructed for that to hold.

It SHALL write the source file back in place as `edit` does, `--output` sending the result elsewhere.

#### Scenario: A form edited outside tCal is folded back
- GIVEN a calendar projected with `template` and edited by anything at all
- WHEN `apply` is given that document and that calendar
- THEN the result is what `edit` would have written, byte for byte

### Requirement: A fold back with nobody to ask is an error
A document that does not parse, and one leaving a collision undecided, SHALL fail naming what could not be folded, rather than offer a re-edit. `edit` asks because a person is sitting in front of it; `apply` has nobody to ask, and a prompt in a pipeline is a hang.
