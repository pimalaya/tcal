---
cairn: log
change: the-editor-is-spawned-here
landed: 2026-09-01
---

# The editor is spawned here, and named when it cannot be found

tCal opened the editor through the [edit](https://crates.io/crates/edit) crate, and inherited its fallback list: with neither `$VISUAL` nor `$EDITOR` set, that crate walks down to `xdg-open`, `gnome-open`, `kde-open` and a bare `open` (edit-0.1.5/src/lib.rs:49-72). Those are file openers, not editors. They hand the `.toml` to whatever the desktop associates with it and return while the window is still up, so tCal read back a document nobody had touched and wrote the calendar out exactly as it went in, reporting nothing. A caller spawning tCal reads a document handed back untouched as an edit given up on, so an unset `$EDITOR` on a desktop became a silently abandoned edit.

**The resolution is three lines and a refusal** (src/cli/editor.rs): `--editor`, then `$VISUAL`, then `$EDITOR`, then `No editor found; set $VISUAL or $EDITOR, or pass --editor <COMMAND>`. tCal picks no editor on anyone's behalf.

**`--editor <COMMAND>`** (src/cli/args.rs) is a shared argument on `edit` and `merge`, spelled as tCard spells it.

**The spawn is ours**: `tcal-<uuid>.toml` in the temporary directory, the command line split on whitespace so `code --wait` carries its argument, the path appended last, the three streams inherited, the file read back on exit. The `edit` dependency and its feature entry are gone.

**The buffer outlives the run that could not use it**: a document that does not fold back and is not re-edited keeps its file, named as `Cannot fold back <path>`, and so does an editor exiting non-zero. An editor that could not be spawned removes it, and a fold that succeeded removes it too. tCal's own two questions are unchanged, the TOML parse failure and the undecided property each keeping their wording.

This is [tCard's change of the same name](https://github.com/pimalaya/tcard) ported: the two projects are one design over two formats, and an editor round trip is exactly the sort of thing that must not drift between them.

Capabilities moved: editor, a new capability file, four requirements. Nothing in reading, template, merge or api changed.

Verified with `--editor` winning over `$VISUAL`, `$VISUAL` over `$EDITOR`, `$EDITOR` alone, an unset pair failing and spawning nothing, a missing program removing the file, a non-zero exit keeping and naming it, and a successful fold leaving the temporary directory clean. 85 tests green on the full feature set, clippy clean.
