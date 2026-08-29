---
cairn: log
change: parse-once
date: 2026-08-29
---

# Two libraries read every calendar, and the second one lost bytes

Every body went through both ical-rs and calcard: the merge reconciled a syntax tree, serialised it, and calcard parsed the result again so the projection had a typed calendar to walk. One body, two readers, two models, and the second one normalised what it read. `CATEGORIES:a\,  b` came back `a\, b`, an absolute alarm trigger came back respelled with separators the calendar never wrote, and the loss happened before `template::project` saw anything, which is handed a parsed calendar rather than the source bytes.

## What landed

- **calcard is gone.** `crate::ical` is the one reader: `parse` reads a whole stream into ical-rs's `IcalCst`, and the projection, the merge and the fold-back all walk that same tree. The dependency is dropped and `merge` no longer pulls a second iCalendar library in, so the feature now gates only the reconciliation.

- **src/edit is gone with it**, 844 lines of in-house format-preserving editor replaced by ical-rs's tree, which is the same design maintained next door and now fuzzed. What is left is 325 lines in src/ical.rs: the stream type, the two edits a fold-back makes (`Component::set_lines`, `Container::set_child_count`) and the line building behind them. An unchanged line is left where it stands, so its folds, its parameter casing and its ending are the source's own; a line the document moved is written anew, unfolded, which RFC 5545 3.1 permits and which is what ical-rs does with an edited line.

- **A file's other calendars survive.** ical-rs's single-calendar `parse` drops everything after the first, so `parse` reads with `parse_many` and keeps the stream whole: the first is the one every verb reads, the rest are carried through byte for byte. A new law pins it, and one fixture (libical_calendar.ics, three calendars) would have silently lost two of them.

- **A merged calendar is projected as the merge left it.** `Merge::project` no longer serialises the merged tree and reads it back; the tree goes straight to `Sides`, and its three inputs are the trees the merge was given. The text is still produced, as the source `apply` patches, but nothing reads it.

- **A line is read through its logical form.** ical-rs ends a line's head at the first colon, quoted parameter values included, so `DESCRIPTION;ALTREP="cid:part1":a description` splits in the wrong place. Its bytes still round-trip, so `logical` reconstructs the line exactly, and `util` re-splits it with the same quote-aware grammar `patch` already used to write one back. tcal therefore owns the content-line grammar it patches, escaping included (`escape` gained its inverse `unescape`), and ical-rs owns the calendar structure and the bytes.

## What the model swap cost

Nothing, in the end: every golden fixture projects to the same TOML it did before, the `.lossy` markers are unchanged, and no field went missing. The typed model calcard supplied was only ever used to re-render a value the calendar had already written, so reading the raw value is both simpler and more faithful. `Kind::Offset` no longer reassembles `±HHMM` from a parsed date-time, `Kind::Date` reads the digit form directly, and a value in a form the model does not read is shown as the calendar wrote it rather than as a reader guessed it.

## Verification

- The whole suite green: 53 lib, 16 merge forcing (1 still ignored, blocked in the merge crate), 7 projection laws, the fixture database, the doctest. All four configurations build (`--all-features`, `--no-default-features`, `--no-default-features --features merge`, still `no_std`), `clippy --all-features --all-targets` is clean, `cargo fmt` run.
- `an_escape_in_a_list_item_keeps_the_space_behind_it` is no longer ignored, and the projection generators dropped the filter that kept escapes out of list items, so every law now runs on them.
- `trigger_raw_fallback_for_date_time` asserts the whole calendar comes back byte for byte, where it used to assert the respelling calcard produced.
- Two laws are new: a file's other calendars survive an edit to the first, and the escape reproduction above.

Capabilities moved: reading (new, ADDED: One reader per body); merge (MODIFIED: Merging is a verb over three files).
