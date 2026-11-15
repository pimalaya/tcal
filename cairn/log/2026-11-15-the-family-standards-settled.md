---
cairn: log
change: the-family-standards-settled
landed: 2026-11-15
---

# The family standards, settled

The owner's decisions of 2026-11-15 on the points the-family-standards left open, taken with tCard's (see its log of the same name).

## What landed

- A document written to a file stays silent in text and answers its receipt under `--json`; output.md already gives the composer reason, unchanged.
- `cli` stays the one default feature; README.md and CONTRIBUTING.md no longer call it opt-in.
- `edit` and `merge` under `--json` fail coded `usage-invalid`, exit 2, before an editor opens (src/cli/editor.rs). `editor-missing` and `editor-failed` left the code vocabulary: text output prints no code, so nothing could carry them.
- `apply - -` exits 2: pimalaya-cli 3bada99 exits 2 for any failure coded `usage-invalid`.
- Folding at 75 octets stays, with its CHANGELOG line.
- The editor re-opens on `TcalError::is_document_refusal` (TOML, undecided collision, unreadable date), the rule tCard now states the same way; the prompts moved into `reprompt`, as in tCard. Behaviour unchanged: these were already the three.
- MIGRATION.md, from v0.2 to v0.3.

## Capabilities moved

- [editor](../spec/editor.md): *The editor re-opens on a document it cannot fold* and *No editor runs under JSON* added.
- [output](../spec/output.md): the two editor codes removed; *A refused command line exits 2* covers `apply - -` and an editor under JSON.
