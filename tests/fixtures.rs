//! Golden fixture tests.
//!
//! Each `tests/data/<name>.<mode>.toml` is the expected projection of
//! `tests/data/<name>.ics` for `<mode>` (`all` for the whole calendar, or
//! `_`-joined component-type keys like `event` or `event_todo`). One `.ics`
//! can have several expectations. To add a case (e.g. from a bug report),
//! drop the `.ics` in and generate the `.toml` with `tcal template`.

use std::{fs, path::Path};

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
        let calendar = tcal::ical::parse(&ics).unwrap();

        let projected = tcal::template::project_with(&calendar, &flags(mode)).unwrap();
        assert_eq!(
            projected,
            expected,
            "projection mismatch: {}",
            path.display()
        );

        // The untouched projection folds back onto the source byte-for-byte.
        let round_trip = tcal::template::apply_with(&ics, &expected, &flags(mode)).unwrap();
        assert_eq!(round_trip, ics, "round-trip mismatch: {}", path.display());
    }
}
