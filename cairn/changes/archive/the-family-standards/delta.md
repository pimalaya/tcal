---
cairn: change
id: the-family-standards
status: landed
created: 2026-11-14
---

# Delta

## ADDED Requirements

Into spec/output.md, a new capability: A document prints as itself, or as its contents under JSON; A document written to a file answers its receipt under JSON; A failure carries a stable code; A refused command line exits 2; Every verb has a schema. Their text is the one spec/output.md holds.

Into spec/template.md:

### Requirement: An edited long line is folded
A line the document moves SHALL be written anew by ical-rs, folded at 75 octets as RFC 5545 section 3.1 asks, never inside a UTF-8 sequence. A line the document leaves alone SHALL keep the layout it was written in.

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
