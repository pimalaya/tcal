---
cairn: log
change: prefix-the-library
date: 2026-08-30
---

# The library carries its prefix, the CLI does not

`Calendar`, `Container`, `Component`, `Prop`, `Template`, `Merge`, `Merged` and the `Result` alias became `TcalCalendar`, `TcalContainer`, `TcalComponent`, `TcalProp`, `TcalTemplate`, `TcalMerge`, `TcalMerged` and `TcalResult`, joining the `TcalError` that already carried it. Nothing under cli moved: `Cli`, `Command`, the three `*Command`s, `SourceArg`, `ComponentFlags`, `Editor` and `Output` are bare, which is the override cli-001 grants and not an oversight.

The line the rule draws is the `cli` feature gate, which makes it checkable rather than a matter of taste: what ships to a library consumer is prefixed, what only the binary sees is not.

## What the rename walked into

Two of the names are ical-rs's own vocabulary: `IcalItem::Component` and `IcalItem::Prop` are variants of a foreign enum and had to be left exactly as they are, which a word-boundary rename does not know. They were fenced off before the pass and restored after it.

The English words were the other trap. Module headers and doc sentences opening with "Merge" or "Template" came out prefixed, and all are back to prose. The module names themselves never moved, so `crate::merge::TcalMerge` is the path, and the header of that module is still "# Merge".

## The version the patch was pointing at

The compile errors that came out of the rename were not the rename's. ical-rs had been bumped to 0.3.0 in the working copy, while the manifest still asked for `"0.2"`, so cargo stopped applying the path patch and silently fell back to the registry's 0.2.0, a different API. The requirement is now `"0.3"` and the patch applies again. A path patch whose version no longer satisfies the requirement fails this quietly, which is worth knowing the next time the twin repositories move apart.

## Verification

The suite is green unchanged in what it asserts, 77 tests, plus clippy, rustdoc with no broken link and both feature builds.

Capabilities moved: `api`.
