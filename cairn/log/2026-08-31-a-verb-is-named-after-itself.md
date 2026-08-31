---
cairn: log
change: a-verb-is-named-after-itself
date: 2026-08-31
---

# Two of the three verbs did not exist under the names everything gave them

`prefix-the-library` was applied one level too deep, and `Command::TcalTemplate` and `Command::TcalMerge` took the prefix meant for the library alone. clap names a subcommand after its variant, so the binary answered to `tcal-template` and `tcal-merge`. `tcal template` was an error, and `infer_subcommands` could not rescue it: inference works on prefixes, and `template` is not a prefix of `tcal-template`. `Edit` had been left alone, so one verb of three still worked, which is why nothing looked broken at a glance.

Every document said otherwise. The README's command lines, the fixture recipe in CONTRIBUTING and every changelog entry describing the verbs all name them bare.

The variants are now `Template` and `Merge`. A test reads the names off `Cli::command()` and asserts the three verbs are there, so the surface is checked rather than assumed, which is what was missing: the variant names are the CLI's public surface, and no test had ever read them.

Capabilities moved: api.
