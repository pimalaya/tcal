---
cairn: log
change: one-section-per-kind
landed: 2026-08-29
---

# One section per kind, because a release is a net diff

The unreleased section carried two Added headings with a Changed heading between them: three additions above it and seven below. Nothing shipped yet, so that section is the whole first release, and a reader who took the first heading for the additions would have stopped one heading short of the projection library, the CLI and five of the vocabulary entries.

## What landed

The two Added headings are one, holding all ten entries in the order they stood, first block then second, with the architecture header entry after them. Changed follows with its two entries, unchanged. No entry was rewritten, dropped or merged into another: the shape moved, the content did not.

The rule behind it is now a requirement of the documentation capability, so the next entry has one place to go.

## Verification

Every entry present before is present after, and the section holds one Added heading and one Changed heading.

Capabilities moved: documentation (ADDED: The changelog is one net diff).
