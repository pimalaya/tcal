---
cairn: log
change: a-fold-back-is-a-verb
landed: 2026-09-01
---

# A fold back is a verb, not a step inside the editor

tCal projected a calendar to TOML with `template` and came back only through `edit`. The way out was a verb anyone could pipe; the way in belonged to the editor tCal spawned. A form edited by a `sed` line, a script, a graphical app or a person on another machine had no way back, even though `TcalTemplate::apply` is public and `edit` is a thin wrapper over it.

**`apply <TEMPLATE> [SOURCE]`** (src/cli/apply.rs) closes it: the edited document, the calendar it was projected from, the folded-back iCalendar out. `-` reads the document from stdin, and both inputs being stdin is refused rather than raced. It carries the same component-type flags as `template`, because a type the form does not show is one the fold back leaves alone, and the same template has to be reconstructed for that to hold. It writes the source file back in place as `edit` does, `--output` sending the result elsewhere.

**A refusal is an error, not a question**: broken TOML and an undecided collision both fail naming what could not be folded. `edit` asks because a person is sitting there; `apply` has nobody to ask, and a prompt in a pipeline is a hang.

**Nothing else moved.** The fold back itself is the library's, unchanged, so `apply` and `edit` write the same bytes for the same document.

This is [cardamum's `card build`](https://github.com/pimalaya/cardamum) one layer down: the pipeline half of an interactive verb, same source, same inputs, no interaction. tCard takes the same verb in the same terms.

Capabilities moved: template (two requirements added).

Verified against a card carrying `CATEGORIES;LANGUAGE=fr:travail,equipe`: a summary edited in the form comes back with that line byte for byte, the document reads from a path or from stdin, both stdin is refused, broken TOML fails with the parser's own message, and a filled blank form mints an event with a fresh `UID`. 85 tests green on the full feature set, clippy clean.
