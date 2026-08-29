---
cairn: tasks
change: parse-once
---

- [x] Confirm ical-rs's model covers every property and component the projection shows
- [x] Port template/model.rs and template/mod.rs onto ical-rs's model
- [x] Fold back through ical-rs's tree layer, retiring src/edit and keeping patch.rs's behaviour
- [x] Drop the calcard dependency
- [x] Un-ignore the escape reproduction, which closes with the reader that caused it
- [x] Drop the list-item filter the projection generators carry for the escape bug
- [x] Verify projection equality and byte-exact round-trip across the whole fixture corpus
- [x] Verify zoned dates keep their bytes, the multi-line contest and the `-tz` arm included
- [x] Verify the four build configurations and that the crate stays no_std
