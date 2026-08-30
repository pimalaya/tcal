---
cairn: delta
change: local-on-the-left
---

## MODIFIED Requirements

### Requirement: Merging is a verb over three files

`merge` SHALL take a base, a local and a remote calendar as paths, plus the path to write, run the three-way merge in process, and project the result as TOML for editing. It SHALL write the output path only once the edited document parses, and SHALL leave it untouched otherwise.

The capability SHALL be built unconditionally. ical-rs is a plain dependency of every configuration, so gating the merge changes nothing about the crate set and a cargo feature has nothing left to buy.

## REMOVED Requirements

### Requirement: The edited side speaks for an address

Removed. `merge` no longer takes the calendar address the edited side speaks for, and no change is refused for want of organiser authority, ical-rs having dropped the capability.
