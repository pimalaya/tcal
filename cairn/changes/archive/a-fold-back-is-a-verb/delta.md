---
cairn: change
id: a-fold-back-is-a-verb
status: landed
created: 2026-09-01
---

# Delta

## ADDED Requirements

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

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
