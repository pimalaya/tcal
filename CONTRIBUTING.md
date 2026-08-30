# Contributing guide

Thank you for investing your time in contributing to tCal.

Whether you are a human or an AI agent, read these in order before touching the code:

1. the [Pimalaya README](https://github.com/pimalaya) for what the project is and how its repositories stack;
2. the [Pimalaya CONTRIBUTING](https://github.com/pimalaya/.github/blob/master/CONTRIBUTING.md) guide, which chains to the shared architecture and guidelines;
3. the inline header documentation in [src/lib.rs](./src/lib.rs): it is the architecture document of this crate;
4. the [cairn](./cairn) folder for the living specification, the in-flight proposals and the landed history, activated by [AGENTS.md](./AGENTS.md).

Everything below documents only what differs from the Pimalaya standards.

## Feature matrix

tcal speaks no protocol, so the layered build of the org guide does not apply here. It has one feature, `cli`, off by default: it carries the binary, clap, the editor and the clock, and it is the only thing pulling in the standard library.

Build both, so that nothing std-only leaks into the `no_std` core:

```sh
cargo build                                # the library alone, no_std over alloc
cargo build --features cli                 # the library and the binary above it
```

The manifest patches ical-rs to its git repository, the projection tracking that syntax tree too closely to sit on a release. Point the patch at a working copy when changing both at once:

```sh
cargo test --all-features --config 'patch.crates-io.ical-rs.path="../ical"'
```

## Adding a fixture

tests/data is a golden database of calendars, described in the golden fixture database section of the [src/lib.rs](./src/lib.rs) header. A real-world calendar is the fastest way to turn a bug report into a regression test:

1. drop the calendar in as tests/data/NAME.ics;
2. generate the expectation beside it with the command below, adding the type flags and naming the file after them, `_`-joined, when the case is a narrowed projection;
3. read the generated TOML: anything wrong in it is a bug in the code, not in the fixture;
4. add an empty tests/data/NAME.lossy marker when the source will not round trip byte for byte, the known limitations section of the [src/lib.rs](./src/lib.rs) header saying when it will not;
5. run the tests.

```sh
cargo run --features cli -- template tests/data/NAME.ics -o tests/data/NAME.all.toml
```
