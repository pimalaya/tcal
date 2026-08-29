---
cairn: tasks
change: parse-once
---

- [ ] Confirm ical-rs's model covers every property and component the projection shows
- [ ] Port template/model.rs and template/mod.rs onto ical-rs's model
- [ ] Fold back through ical-rs's tree layer, retiring src/edit and keeping patch.rs's behaviour
- [ ] Drop the calcard dependency
- [ ] Un-ignore the escape reproduction, which closes with the reader that caused it
- [ ] Drop the list-item filter the projection generators carry for the escape bug
- [ ] Verify projection equality and byte-exact round-trip across the whole fixture corpus
- [ ] Verify zoned dates keep their bytes, the multi-line contest and the `-tz` arm included
- [ ] Verify the four build configurations and that the crate stays no_std
