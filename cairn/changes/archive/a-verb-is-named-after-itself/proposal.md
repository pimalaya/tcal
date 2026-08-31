---
cairn: change
id: a-verb-is-named-after-itself
status: landed
created: 2026-08-31
---

# Two of the three verbs do not exist under the names everything gives them

## Why

`prefix-the-library` asked every pub item outside the cli module to carry the `Tcal` prefix and every item inside it to carry none. It was applied one level too deep: `Command::TcalTemplate` and `Command::TcalMerge` took the prefix too.

clap derives a subcommand's name from its variant, so the binary offered `tcal-template` and `tcal-merge`. `tcal template` was an error, and `infer_subcommands` could not reach the real name from it, since inference works on prefixes and `template` is not a prefix of `tcal-template`. Only `Edit` was left alone, so one verb of three still worked.

Every document in the repository says otherwise: the README's command lines, CONTRIBUTING's fixture recipe, and every changelog entry describing the verbs.

Nothing caught it. The variant names are the CLI's public surface and no test read them.

## What

- Rename the two variants to `Template` and `Merge`.
- Pin the verb names in a test that reads them off the built clap command, so the surface is checked rather than assumed.
