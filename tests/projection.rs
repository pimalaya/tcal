//! # Projection laws
//!
//! Property-based laws of the TOML projection, held over generated
//! calendars and over every golden fixture.
//!
//! The projection is only trustworthy if folding it back is a no-op: an
//! untouched document must leave the calendar exactly as it was, projecting
//! the folded calendar again must give the very same document, and a
//! property the vocabulary does not model must come out byte for byte.
//!
//! The generator below builds calendars out of the modelled vocabulary in
//! the spelling the projection writes back, so a failure is the
//! projection's and not the writer's.

use proptest::prelude::*;
use tcal::template::TcalTemplate;

/// Read a calendar the way the CLI does, with no component filter.
fn template(src: &str) -> TcalTemplate<'_> {
    TcalTemplate::parse(src).unwrap()
}

/// Fold an untouched projection of a calendar back onto its own source.
fn round_trip(src: &str) -> String {
    let template = template(src);

    template.apply(&template.project()).unwrap()
}

/// Project a calendar with no component filter.
fn project(src: &str) -> String {
    template(src).project()
}

/// Wrap event properties into an iCalendar file.
fn calendar(lines: &[String]) -> String {
    let mut out = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n\
         BEGIN:VEVENT\r\nUID:e1@example\r\nDTSTAMP:20260101T000000Z\r\n\
         DTSTART:20260105T090000Z\r\n",
    );

    for line in lines {
        out.push_str(line);
        out.push_str("\r\n");
    }

    out.push_str("END:VEVENT\r\nEND:VCALENDAR\r\n");
    out
}

/// Escape a text value the way RFC 5545 section 3.3.11 asks.
///
/// That is the spelling the projection writes one back in.
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(',', "\\,")
        .replace(';', "\\;")
}

/// A non-empty text value.
///
/// Its alphabet includes every character RFC 5545 escapes, so escaping is
/// exercised rather than avoided.
fn value() -> impl Strategy<Value = String> {
    proptest::collection::vec(
        prop_oneof![
            proptest::char::range('a', 'z'),
            proptest::char::range('A', 'Z'),
            proptest::char::range('0', '9'),
            Just(' '),
            Just('-'),
            Just('.'),
            Just('\''),
            Just('é'),
            Just(','),
            Just(';'),
            Just('\\'),
            Just(':'),
        ],
        1..10usize,
    )
    .prop_map(|chars| chars.into_iter().collect::<String>().trim().to_owned())
    .prop_filter("a value is not empty", |text| !text.is_empty())
}

/// A calendar address, which the projection writes without its scheme.
fn address() -> impl Strategy<Value = String> {
    "[a-z]{3,6}@example\\.com".prop_map(String::from)
}

/// A property name the vocabulary does not model.
///
/// The projection never shows one, and apply must keep it untouched.
fn extension() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("X-CUSTOM".to_owned()),
        Just("X-VENDOR-THING".to_owned()),
        Just("CONTACT".to_owned()),
        Just("RELATED-TO".to_owned()),
        Just("ATTACH".to_owned()),
    ]
}

/// One unmodelled property line, parameters and all.
fn extension_line() -> impl Strategy<Value = String> {
    (extension(), prop::option::of(value()), value()).prop_map(|(name, param, text)| match param {
        Some(param) => format!("{name};X-P={};VALUE=TEXT:{}", escape(&param), escape(&text)),
        None => format!("{name}:{}", escape(&text)),
    })
}

prop_compose! {
    /// An iCalendar file holding one event.
    ///
    /// The event is built from the modelled vocabulary, plus a few
    /// properties outside it.
    fn ical()(
        summary in value(),
        description in prop::option::of(value()),
        location in prop::option::of(value()),
        categories in prop::collection::vec(value(), 0..3),
        priority in prop::option::of(1..9u8),
        status in prop::option::of(prop::sample::select(vec!["TENTATIVE", "CONFIRMED", "CANCELLED"])),
        class in prop::option::of(prop::sample::select(vec!["PUBLIC", "PRIVATE", "CONFIDENTIAL"])),
        organizer in prop::option::of(address()),
        language in prop::option::of(prop::sample::select(vec!["en", "fr"])),
        attendees in prop::collection::vec(
            (
                address(),
                prop::sample::select(vec!["NEEDS-ACTION", "ACCEPTED", "DECLINED"]),
                prop::option::of(prop::sample::select(vec!["TRUE", "FALSE"])),
            ),
            0..3,
        ),
        alarms in prop::collection::vec(1..60u8, 0..3),
        extensions in prop::collection::vec(extension_line(), 0..3),
    ) -> String {
        // NOTE: the language of a summary and an attendee's RSVP are
        // parameters of a modelled property the projection never shows, so
        // they are generated to hold every law to them as well.
        let mut lines = match &language {
            Some(language) => vec![format!("SUMMARY;LANGUAGE={language}:{}", escape(&summary))],
            None => vec![format!("SUMMARY:{}", escape(&summary))],
        };

        if let Some(description) = description {
            lines.push(format!("DESCRIPTION:{}", escape(&description)));
        }
        if let Some(location) = location {
            lines.push(format!("LOCATION:{}", escape(&location)));
        }
        if !categories.is_empty() {
            let joined: Vec<String> = categories.iter().map(|item| escape(item)).collect();
            lines.push(format!("CATEGORIES:{}", joined.join(",")));
        }
        if let Some(priority) = priority {
            lines.push(format!("PRIORITY:{priority}"));
        }
        if let Some(status) = status {
            lines.push(format!("STATUS:{status}"));
        }
        if let Some(class) = class {
            lines.push(format!("CLASS:{class}"));
        }
        if let Some(organizer) = organizer {
            lines.push(format!("ORGANIZER:mailto:{organizer}"));
        }
        for (address, partstat, rsvp) in &attendees {
            let rsvp = rsvp.map(|rsvp| format!(";RSVP={rsvp}")).unwrap_or_default();
            lines.push(format!("ATTENDEE{rsvp};PARTSTAT={partstat}:mailto:{address}"));
        }

        lines.extend(extensions);

        for minutes in &alarms {
            lines.push(format!(
                "BEGIN:VALARM\r\nACTION:DISPLAY\r\nTRIGGER:-PT{minutes}M\r\nEND:VALARM",
            ));
        }

        calendar(&lines)
    }
}

proptest! {
    /// An untouched projection folds back onto the calendar it came from.
    ///
    /// Every other law rests on this one. The calendar is written in the
    /// form the projection writes back, so there is nothing to renormalise
    /// and the comparison can be exact.
    #[test]
    fn folding_an_untouched_projection_changes_nothing(src in ical()) {
        prop_assert_eq!(round_trip(&src), src.clone());
    }

    /// Projecting a folded projection gives the very same document.
    ///
    /// The projection settles at once rather than converging over repeated
    /// edits, so a reader never sees a calendar move under them.
    #[test]
    fn projecting_a_folded_projection_gives_an_identical_document(src in ical()) {
        let once = round_trip(&src);
        prop_assert_eq!(project(&once), project(&src));
        prop_assert_eq!(round_trip(&once), once.clone());
    }

    /// An unmodelled property is never shown and never touched.
    ///
    /// It comes out of the round trip exactly as it went in, and the
    /// projection never names it.
    #[test]
    fn an_unmodelled_property_survives_verbatim(src in ical()) {
        let toml = project(&src);
        let out = round_trip(&src);

        for line in src.lines().filter(|line| is_unmodelled(line)) {
            let name = line.split([':', ';']).next().unwrap();
            prop_assert!(!toml.contains(name), "{} is shown in the projection", name);
            prop_assert_eq!(
                out.matches(line).count(),
                src.matches(line).count(),
                "{} did not survive as it was",
                line,
            );
        }
    }
}

/// Whether a content line writes a property the vocabulary does not model.
fn is_unmodelled(line: &str) -> bool {
    let name = line.split([':', ';']).next().unwrap_or(line);

    matches!(
        name,
        "X-CUSTOM" | "X-VENDOR-THING" | "CONTACT" | "RELATED-TO" | "ATTACH"
    )
}

/// Every golden fixture survives repeated round trips.
///
/// One pass settles the calendar, and a second changes nothing.
#[test]
fn every_fixture_settles_after_one_round_trip() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/data");

    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();

        if path.extension().is_none_or(|ext| ext != "ics") {
            continue;
        }

        let src = std::fs::read_to_string(&path).unwrap();
        let once = round_trip(&src);

        assert_eq!(round_trip(&once), once, "drifts: {}", path.display());
        assert_eq!(project(&once), project(&src), "loses: {}", path.display());
    }
}

/// A modelled property keeps the parameters the projection does not show.
///
/// An unmodelled property keeps everything the same way.
#[test]
fn a_modelled_property_keeps_its_unshown_parameters() {
    for line in [
        "DESCRIPTION;ALTREP=\"cid:part1\":a description",
        "ORGANIZER;CN=Chair;SENT-BY=\"mailto:sec@example.com\":mailto:chair@example.com",
        "ATTENDEE;RSVP=TRUE;CUTYPE=INDIVIDUAL;PARTSTAT=ACCEPTED:mailto:ada@example.com",
        "SUMMARY;LANGUAGE=en:a summary",
    ] {
        let src = calendar(&[line.to_owned()]);
        assert_eq!(round_trip(&src), src);
    }
}

/// An escape inside a multi-valued item comes back as it was.
///
/// The space behind it is not eaten: one reader reads the value, and the
/// escaping it undoes is the one the projection puts back.
#[test]
fn an_escape_in_a_list_item_keeps_the_space_behind_it() {
    for escaped in ["\\,", "\\;", "\\\\"] {
        let src = calendar(&[format!("CATEGORIES:a{escaped}  b")]);

        assert_eq!(round_trip(&src), src);
    }
}

/// A file holding more than one calendar keeps the ones never shown.
///
/// Only the first is read, and the rest come out as they went in.
#[test]
fn a_calendar_beside_the_one_being_read_survives() {
    let second = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Other//EN\r\n\
        BEGIN:VEVENT\r\nUID:e2@example\r\nDTSTAMP:20260101T000000Z\r\n\
        SUMMARY:Untouched\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
    let src = format!("{}{second}", calendar(&["SUMMARY:Lunch".to_owned()]));

    assert_eq!(round_trip(&src), src);

    let edited = project(&src).replace("Lunch", "Team lunch");
    let out = template(&src).apply(&edited).unwrap();

    assert!(out.contains("SUMMARY:Team lunch"), "{out}");
    assert!(out.ends_with(second), "{out}");
}
