---
cairn: change
id: structures-over-functions
status: landed
created: 2026-08-30
---

# The library is a bag of functions, and the reader has to hold the arguments

## Why

tCard answered this one first, and tCal is its twin: what lands there lands here, as merge, prefer-local and the identity contest already did.

Every entry point of the projection is a free function over positional arguments: `ical::parse(input)`, `template::project(calendar)`, `template::project_with(calendar, types)`, `template::project_one(calendar, ty)`, `template::apply(original, edited)`, `template::apply_with(original, edited, types)`. Six functions for two directions and one filter, and a caller has to keep the same calendar, the same source text and the same type list matched up across two of them by hand.

`template::apply` takes the original text back because the projection did not keep it, so the two halves of one round trip are two functions that a caller has to remember to give the same calendar twice. A type holding the calendar and its type filter makes that structural: the tree the form was projected from is the tree the fold-back patches, which is what the one-reader-per-body requirement asks for anyway, and the filter the projection showed is the filter the fold-back reconciles.

The reading of a calendar is free functions too. `ical::props`, `ical::children`, `ical::nested`, `ical::named` and `ical::logical` all take a syntax node as their first argument and sit in a module that already declares two traits over that very node, and `template::util` holds four more that read one content line.

The two big modules are the other half of it. cli.rs carried three commands, their shared arguments, the editor round trip and the file writing in one file, and merge.rs carried the sides, the choice, the notes and the document in another, at 1090 lines. Neither is navigable by the name of the thing you are looking for.

Two smaller things ride along. thiserror is a dependency for one enum of five variants whose Display is five lines by hand, and ical-rs already writes its own. And the product is tCal, while `tcal` is the identifier: the document a person edits says "edited by tcal", which is the binary's name pretending to be the product's.

## What

- Give every entry point a type that names its inputs: `Calendar::parse`, `Template { calendar, types }`, and the `Merge` and `Merged` the merge already had.
- Fold `template::apply` onto the template it was projected from, so a round trip is two methods on one value rather than two functions over the same calendar twice.
- Move the reading of a component and of a content line onto the `Component`, `Container` and `Prop` traits, which is where the rest of that module already was.
- Split cli.rs into one module per verb over shared argument and editor modules, and merge.rs into sides, choice and document under a facade.
- Drop thiserror and write Display and Error by hand.
- Write the product name as tCal everywhere it is prose, keeping `tcal` for the crate, the binary and the module.
- Cut the inline comments that narrate what the code says, keeping the ones that carry a why nothing else records.
- Stamp a line a fold-back builds with the escaping rules the calendar's own version declares, rather than the default, now that ical-rs gives a parameter node one.
