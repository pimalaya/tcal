---
cairn: change
id: a-fold-back-is-a-verb
status: landed
created: 2026-09-01
---

# A fold back is a verb, not a step inside the editor

## Why

tCal projects a calendar to TOML with `template`, and comes back only through `edit`. The way out is a verb anyone can pipe; the way in is available exclusively to the editor tCal spawns. A form edited by anything else, a `sed` line, a script, a graphical app, a person on another machine, cannot be folded back at all, and the library method that would do it, `TcalTemplate::apply`, is public with no verb over it.

That is a gap in the projection, not in the editor. tCal's promise is that a calendar becomes a fillable form and comes back byte-faithful; who filled the form is none of its business. The CLI currently answers "the editor I spawned" to a question the library never asked.

Cardamum reached the same conclusion one layer up. Its `card build` is the pipeline half of `card create -i`: the same source, the same flags, no interaction, output someone can look at or pipe. The half missing here is the same half.

## What

**`apply <TEMPLATE> [SOURCE]`**: the edited document, the calendar it was projected from, and the folded-back iCalendar out. `-` reads the document from stdin, and one of the two inputs may be stdin, not both.

**The type flags are the projection's**, so `apply` takes the same `--event`, `--todo` and friends `template` was given: a type the form does not show is one the fold back must leave alone, and reconstructing the same template is what makes that true.

**A refusal is an error here, not a question.** `edit` can ask whether to re-open on broken TOML or an undecided collision because a person is sitting there. `apply` has nobody to ask, so it fails naming what it could not fold.

**It writes in place, as `edit` does**, `--output` sending the result elsewhere, so the two verbs keep one promise about what happens to the file you name.

## What this is not

Not a second editor path: `apply` spawns nothing and reads no environment variable.

Not a merge verb either. `merge` still projects its own document and decides it, and folding that document back outside the editor is the same `apply` over the calendar it names.
