---
cairn: delta
change: structures-over-functions
---

## ADDED Requirements

### Requirement: An entry point is a type that names its inputs
*Folds into api.md.*

Every public entry point SHALL be a method on a type carrying the inputs as named fields, never a free function over positional arguments. A private helper of a few lines stays a function.

Three strings in a row say nothing about which is which, and a merge given its sides the wrong way round compiles and merges backwards. A type names them at the call site, and the name survives every later argument.

ical-rs settled the same question one version below, `IcalMerge { base, left, right }.merge()` replacing a positional `merge`, and a layer that reads the other way makes the reader change idiom halfway down the stack.

#### Scenario: A merge is given its three calendars
- GIVEN a base calendar and the two sides that diverged from it
- WHEN a merge is asked for
- THEN each calendar is passed by name, and no order of the three is silently valid

### Requirement: Both directions belong to one value
*Folds into api.md.*

Projecting a form and folding one back SHALL be two methods on the same value, holding the calendar and the component types it shows once, rather than functions each taking them again.

The fold-back patches the tree the form was projected from, which is the one-reader-per-body requirement stated in the type: a caller cannot hand the second half a different calendar than the first half read, because it does not hand it one at all. Nor can it reconcile a set of types the form never showed.

#### Scenario: A round trip through the editor
- GIVEN a calendar read once and narrowed to one component type
- WHEN it is projected, edited and folded back
- THEN the same value serves both directions, the calendar is parsed once, and the types it showed are the types it reconciles

### Requirement: A reading of a foreign node is a trait method
*Folds into api.md.*

A reading of ical-rs's syntax tree SHALL be a method on the trait that extends the node it reads, never a free function taking that node as its first argument.

A module holding both a trait over a type and a free function whose first argument is that type has two ways to say one thing. The trait is the one that composes: `component.props(name)` reads left to right through a chain, and the import that brings it in brings in all of it.

#### Scenario: Reading a component
- GIVEN a component of a parsed calendar
- WHEN its properties, its children or its name are read
- THEN each is a method on it

### Requirement: A verb is a module
*Folds into api.md.*

Each command of the CLI SHALL live in its own module under cli, over shared modules for the arguments several verbs take and for the editor round trip. A module of the merge SHALL likewise hold one part of it.

A file holding every verb of a CLI, or every part of a merge, is navigable by scrolling and by nothing else. The rule is the one the rest of Pimalaya already follows, and it makes the name of the thing you are looking for the name of the file it is in.

#### Scenario: Looking for what a verb does
- GIVEN the source tree
- WHEN the behaviour of one command is looked for
- THEN it is in the module named after that command

### Requirement: The error enum is written by hand
*Folds into api.md.*

The crate error SHALL implement Display and Error by hand rather than derive them from a dependency.

One enum of five variants is five match arms, and the crate below it writes its own the same way. A derive dependency earns its place where the enum is large or the source chaining is intricate, and this one is neither.

#### Scenario: The dependency list of the core
- GIVEN the library built with no features
- WHEN its dependencies are read
- THEN no error-derive crate is among them

### Requirement: The product is written tCal
*Folds into documentation.md.*

Prose SHALL write the product name tCal, and `tcal` SHALL be reserved for the identifier: the crate, the binary, the module path, a shell command and the `PRODID` a new calendar carries. The document a person edits carries the prose form.

The name is the one thing a reader sees before anything else, in the header of every document the tool generates. Writing the identifier there says the tool is its own binary name, which is a detail of packaging rather than what the thing is called.

#### Scenario: The header of a generated document
- GIVEN a projected TOML form
- WHEN its header comment is read
- THEN it names tCal

## MODIFIED Requirements

None.

## REMOVED Requirements

None.
