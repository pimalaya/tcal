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

/// Two properties of one repeatable name keep their own lines.
///
/// The form shows their items as one array, so a fold-back has to spread it
/// back over the lines it came from: collapsing them into one would drop the
/// second line and the parameters the form never showed with it.
#[test]
fn repeated_list_properties_of_one_name_do_not_collapse() {
    let src = calendar(&[
        "CATEGORIES;LANGUAGE=en:a,b".to_owned(),
        "CATEGORIES;LANGUAGE=fr:c".to_owned(),
    ]);

    let once = round_trip(&src);

    assert_eq!(
        once.lines()
            .filter(|line| line.starts_with("CATEGORIES"))
            .count(),
        2,
        "{once}",
    );
    assert_eq!(once, src);
    assert_eq!(round_trip(&once), once);
}

/// Removing one item leaves the items of every other line where they were.
///
/// The line a value came out of is the one whose parameters describe it, so
/// counting items off the front of the array instead relabels the French
/// category as English and drops the French line with it.
#[test]
fn removing_an_item_leaves_the_other_lines_alone() {
    let src = calendar(&[
        "CATEGORIES;LANGUAGE=en:work,travel".to_owned(),
        "CATEGORIES;LANGUAGE=fr:travail".to_owned(),
    ]);

    let edited = project(&src).replace("\"work\", \"travel\", ", "\"work\", ");
    let out = template(&src).apply(&edited).unwrap();

    assert!(out.contains("CATEGORIES;LANGUAGE=en:work\r\n"), "{out}");
    assert!(out.contains("CATEGORIES;LANGUAGE=fr:travail\r\n"), "{out}");
}

/// A line that loses every item is removed rather than given another's.
///
/// A free window reported as busy is the calendar saying the opposite of what
/// it said, which is the cost of taking a line's parameters for items it never
/// carried.
#[test]
fn a_line_that_loses_every_item_is_removed() {
    let src = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//Test//EN\r\n\
        BEGIN:VFREEBUSY\r\nUID:fb1@example\r\nDTSTAMP:20260101T000000Z\r\n\
        FREEBUSY;FBTYPE=BUSY:20260105T090000Z/PT1H,20260105T140000Z/PT1H\r\n\
        FREEBUSY;FBTYPE=FREE:20260105T120000Z/PT2H\r\n\
        END:VFREEBUSY\r\nEND:VCALENDAR\r\n";

    let edited = project(src).replace("\"20260105T090000Z/PT1H\", \"20260105T140000Z/PT1H\", ", "");
    let out = template(src).apply(&edited).unwrap();

    assert!(!out.contains("FBTYPE=BUSY"), "{out}");
    assert!(
        out.contains("FREEBUSY;FBTYPE=FREE:20260105T120000Z/PT2H\r\n"),
        "{out}",
    );
}

/// Renaming an item rewrites the line it was on rather than opening a second.
///
/// An item no line held fills the room the rename left, so a value the reader
/// edited in place stays in place.
#[test]
fn renaming_an_item_rewrites_its_own_line() {
    let src = calendar(&["CATEGORIES:work,travel".to_owned()]);

    let edited = project(&src).replace("\"travel\"", "\"leisure\"");
    let out = template(&src).apply(&edited).unwrap();

    assert!(out.contains("CATEGORIES:work,leisure\r\n"), "{out}");
    assert_eq!(
        out.lines()
            .filter(|line| line.starts_with("CATEGORIES"))
            .count(),
        1,
        "{out}",
    );
}

/// Items added to a property holding one line join that line.
///
/// This is the README's own example, `categories = ["pimalaya", "cli"]` for
/// `CATEGORIES:pimalaya,cli`. One line has nothing to disambiguate, so an
/// added item can only belong to it: writing a second line instead would say
/// the two categories were recorded apart, which is not what was edited.
#[test]
fn items_added_to_a_single_line_join_it() {
    // NOTE: the empty block of every other component type carries the key
    // too, so each case names the spelling its own event block holds.
    let cases = [
        (calendar(&[]), "categories = []"),
        (
            calendar(&["CATEGORIES:pimalaya".to_owned()]),
            "categories = [\"pimalaya\"]",
        ),
    ];

    for (src, held) in cases {
        let edited = project(&src).replacen(held, "categories = [\"pimalaya\", \"cli\"]", 1);
        let out = template(&src).apply(&edited).unwrap();

        assert!(out.contains("CATEGORIES:pimalaya,cli\r\n"), "{out}");
        assert_eq!(
            out.lines()
                .filter(|line| line.starts_with("CATEGORIES"))
                .count(),
            1,
            "{out}",
        );
    }
}

/// Items no line held share one new line, rather than one line each.
///
/// Which line's parameters they should have carried is the question several
/// lines make unanswerable, so they carry none, together.
#[test]
fn items_no_line_held_share_one_new_line() {
    let src = calendar(&[
        "CATEGORIES;LANGUAGE=en:work".to_owned(),
        "CATEGORIES;LANGUAGE=fr:travail".to_owned(),
    ]);

    let edited = project(&src).replacen(
        "[\"work\", \"travail\"]",
        "[\"work\", \"travail\", \"a\", \"b\"]",
        1,
    );
    let out = template(&src).apply(&edited).unwrap();

    assert!(out.contains("CATEGORIES;LANGUAGE=en:work\r\n"), "{out}");
    assert!(out.contains("CATEGORIES;LANGUAGE=fr:travail\r\n"), "{out}");
    assert!(out.contains("CATEGORIES:a,b\r\n"), "{out}");
    assert_eq!(
        out.lines()
            .filter(|line| line.starts_with("CATEGORIES"))
            .count(),
        3,
        "{out}",
    );
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
