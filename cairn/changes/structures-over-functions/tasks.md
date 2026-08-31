---
cairn: tasks
change: structures-over-functions
---

- [x] Replace the free entry points with `Calendar::parse` and `Template`
- [x] Give `Template` both directions and the type filter, so a fold-back patches the tree its form came from
- [x] Move the component and content-line readings onto the `Component` and `Prop` traits
- [x] Split src/cli.rs into args, editor and one module per verb
- [x] Split src/merge.rs into sides, choice and document under the facade
- [x] Drop thiserror, writing Display and Error by hand
- [x] Write the product name as tCal in prose, including the document's own header
- [x] Cut the inline comments that narrate, keeping the whys
- [x] Thread the calendar's escaping rules into a line a fold-back builds
- [x] Verify the suite, clippy, rustdoc and both feature builds
