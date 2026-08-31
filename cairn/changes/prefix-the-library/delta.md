---
cairn: delta
change: prefix-the-library
---

## ADDED Requirements

### Requirement: The library carries its prefix, the CLI does not
*Folds into api.md.*

Every pub item the library ships SHALL carry the `Tcal` domain prefix. Every item under the cli module SHALL carry none.

The library is consumed by name from outside, where `TcalTemplate` says whose template it is in a `use` list of a dozen; naming-007 asks for the prefix and exempts only foreign re-exports and the shared toolkit crates, neither of which this is. It matters most for the traits: `Component` and `Prop` are ical-rs vocabulary too, and a bare one says nothing about which layer it extends. The cli subtree is the override cli-001 grants: nothing there is consumed as a library, and the binary already names itself.

The line falls exactly where the `cli` feature does, so the rule is checkable by the feature gate rather than by taste.

#### Scenario: The public surface of the two halves
- GIVEN the crate built with all features
- WHEN its public items are read
- THEN every one outside cli is prefixed, and none inside it is

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
