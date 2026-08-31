---
cairn: log
change: structures-over-functions
date: 2026-08-30
---

# An entry point names what it is given

The free functions are gone. `Calendar::parse` reads a stream, and `Template { calendar, types }` projects a form and folds one back, replacing `project`, `project_with`, `project_one`, `apply` and `apply_with`. `Merge` and `Merged` were already the shape the rest now takes, tCard having landed the same change hours before this one.

## Both halves of a round trip

`Template` holds the calendar and the component types it shows, so `apply` no longer takes the source text a second time: it clones the tree the form was projected from and patches that. The one-reader-per-body requirement used to rest on every caller passing the same calendar twice, and now it cannot be broken from outside; nor can a fold-back reconcile a set of types the projection never showed, the filter being the same field both directions read.

`with_types` is the one fallible step, resolving the keys a reader names into the specs behind them, so `UnknownComponent` is raised once rather than by each of the four entry points that used to take a key list.

## Reading is a trait

`props`, `children`, `nested` and `named` moved onto `Component`, which is where `lines` and `set_lines` already were, and `Container::children` became `children_mut`, which is what it always returned. `logical` and the three readings of a value that lived in template/util (`raw`, `text`, `items`, `param`) became the new `Prop` trait over a content line, `param` renamed `param_value` where ical-rs's own typed lens already holds the shorter name. `Calendar` gained `top_level`, which was a free function taking one.

## Three files became eleven

cli.rs kept `Cli` and `Command` and gave up the rest: args holds the source, the component flags and the output sink, editor the `$EDITOR` round trip and its re-edit prompt, and template, edit and merge one verb each. merge.rs kept `Merge`, `Merged` and the helpers its children share, and gave sides the reading of a conflict against the four calendars, choice the contested key that reading yields, and document the projection both are written into. `Document` is now a type with a `decorate` method rather than a function taking three slices.

template/util is gone: the TOML side kept a module under its own name, the line readings became `Prop` methods, and the content-line grammar, its RFC 5545 escapes and the `mailto:` scheme a calendar address wears are `patch::Content`, one line with the whole grammar on it.

## Smaller things

thiserror is gone: five variants, five match arms, and `source` returning the TOML parse error. The core builds with one dependency fewer, and the manifest patches ical-rs to the working copy beside it rather than to its git repository, since the release the projection tracks does not exist yet.

A line a fold-back builds is stamped with the escaping rules the calendar's own `VERSION` declares rather than the default, threaded from `Calendar::escaper` down through the reconciliation. Nothing reads those nodes on the way out, since tCal writes a line through its own content-line grammar, so this fixes no visible defect. It stops the tree from saying something about the calendar that the calendar does not say.

The product is written tCal in prose, `tcal` staying the crate, the binary, the module and the `PRODID`. The generated document says so in its own header, which is where a reader meets the name first, so the golden fixtures moved with it.

Thirty-nine inline comments are down to fifteen. What went narrated the line below it or the assertion beside it; what stayed carries something the code cannot say, and two regression tests turned their note into the doc comment that says why they exist.

## Verification

The whole suite is green unchanged in what it asserts, 77 tests over the unit, fixture, forcing and projection layers, plus clippy, rustdoc and both feature builds. The fixtures moved only in the header line naming the product.

Capabilities moved: `api` (new), `documentation`.
