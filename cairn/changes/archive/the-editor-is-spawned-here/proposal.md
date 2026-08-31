---
cairn: change
id: the-editor-is-spawned-here
status: landed
created: 2026-09-01
---

# The editor is spawned here, and named when it cannot be found

## Why

tCal opens the editor through the [edit](https://crates.io/crates/edit) crate, and inherits the one thing about it worth refusing: when neither `$VISUAL` nor `$EDITOR` is set, the crate walks a fallback list ending in `xdg-open`, `gnome-open` and `kde-open` on Linux and a bare `open` on macOS. Those are generic file openers, not editors. They hand the `.toml` to whatever the desktop associates with it, and they return before it is closed.

tCal then reads back a document nobody has touched yet and writes the calendar out exactly as it went in, reporting nothing. A round trip that changed nothing is indistinguishable from an edit someone thought better of, which is the reading a caller spawning tCal takes: cardamum treats a composer handing back the seed untouched as a no. An unset `$EDITOR` on a desktop therefore turns into a silently abandoned edit, in a program that never asked for a fallback.

There is no way around it either, tCal reading no configuration and offering no flag.

tCard settled this the same day, and the two are one design: what tCard learns about putting a document in front of a person, tCal learns too. The rest of what the crate does is a temporary file, a spawn with the streams inherited and a read back, which is thirty lines against `std`.

## What

**Resolve `$VISUAL`, then `$EDITOR`, and stop.** No fallback list, no file opener. When neither is set the command SHALL say so and name the two variables and the flag.

**`--editor <COMMAND>`**, on `edit` and on `merge`, naming the command for one invocation: the twin of tCard's flag of the same name, and of the `--composer` of a caller spawning either.

**The spawn is ours**: a `tcal-<uuid>.toml` in the temporary directory, the command spawned on its path with stdin, stdout and stderr inherited, and the file read back when it exits.

**A buffer that cannot be folded back is kept and named**, on a declined re-edit and on an editor exiting non-zero. An editor that could not be spawned at all takes the file with it, since it holds nothing but what tCal wrote a moment earlier.

**The `edit` dependency goes.**

## What this is not

Not a configuration file: tCal still reads none. Not editor detection either, `code --wait` staying the caller's business. What changes is that tCal no longer picks a non-blocking command on someone's behalf.
