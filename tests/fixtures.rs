//! # Golden fixtures
//!
//! Golden tests of the projection over real-world and crafted calendars,
//! one case per expectation file under tests/data.
//!
//! A case is tests/data/<name>.<mode>.toml, the expected projection of
//! tests/data/<name>.ics for that mode: `all` for the whole calendar, or
//! `_`-joined component-type keys like `event` or `event_todo`. One calendar
//! can carry several expectations, one per mode.
//!
//! To add a case, from a bug report or otherwise, drop the calendar in and
//! generate its expectation with `tcal template`.
//!
//! Projection is deterministic, so equality is asserted for every fixture.
//!
//! Round-trip is asserted only where the source is already in the form the
//! projection writes back, which the absence of a .lossy marker says: an
//! export often orders an RRULE's tokens its own way, which apply then
//! canonicalises.

use std::{fs, path::Path};

use tcal::template::TcalTemplate;

/// The component-type flags a fixture mode selects (`all` = no filter).
fn flags(mode: &str) -> Vec<String> {
    if mode == "all" {
        Vec::new()
    } else {
        mode.split('_').map(str::to_owned).collect()
    }
}

#[test]
fn fixtures_project_and_round_trip() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");

    let mut paths: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "toml"))
        .collect();
    paths.sort();

    assert!(!paths.is_empty(), "no fixtures in {}", dir.display());

    for path in paths {
        let stem = path.file_stem().unwrap().to_str().unwrap();
        let (name, mode) = stem
            .rsplit_once('.')
            .expect("fixture must be named <name>.<mode>.toml");

        let ics = fs::read_to_string(dir.join(format!("{name}.ics"))).unwrap();
        let expected = fs::read_to_string(&path).unwrap();
        let template = TcalTemplate::parse(&ics)
            .unwrap()
            .with_types(&flags(mode))
            .unwrap();

        let projected = template.project();
        assert_eq!(
            projected,
            expected,
            "projection mismatch: {}",
            path.display()
        );

        if !dir.join(format!("{name}.lossy")).exists() {
            let round_trip = template.apply(&expected).unwrap();
            assert_eq!(round_trip, ics, "round-trip mismatch: {}", path.display());
        }
    }
}
