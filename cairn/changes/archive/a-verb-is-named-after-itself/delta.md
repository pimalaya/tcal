---
cairn: delta
change: a-verb-is-named-after-itself
---

## ADDED Requirements

### Requirement: A verb is named after itself
*Folds into api.md.*

The name a verb answers to on the command line SHALL be the verb alone. A `Command` variant SHALL therefore carry no library prefix, and the names the built command tree offers SHALL be checked by a test rather than assumed.

clap takes a subcommand's name from its variant, so the variant names are the CLI's public surface as surely as a pub item is the library's. A prefix there does not read as a namespace, it renames the command: `TcalTemplate` is spelled `tcal-template` on the command line, which no document names and which subcommand inference cannot reach from `template`.

This is the same line `prefix-the-library` drew, read at the right depth: the prefix stops at the `cli` module, and everything the module exposes to a person is inside it.

#### Scenario: The verbs a person types
- GIVEN the command tree the binary parses
- WHEN its subcommand names are read
- THEN they are `template`, `edit` and `merge`

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
