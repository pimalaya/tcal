//! Property-based laws of the merge document's duplicate-key forcing.
//!
//! A collision the projection can address is written as the same TOML key
//! once per side, which TOML refuses, so an undecided document cannot be
//! applied at all. These laws pin that down from both ends: that an
//! undecided document is refused, that keeping either side yields that side,
//! that a value of the reader's own is taken as written, and that a contest
//! inside a nested component never escapes the single block that component
//! projects to, where a repeated array-of-tables header would be legal TOML
//! and would quietly make a second alarm or a second attendee instead of an
//! error.
//!
//! The laws that do not hold today are kept here as ignored tests, each
//! naming the finding that reproduces it.

#![cfg(feature = "merge")]

use proptest::prelude::*;
use tcal::{
    error::TcalError,
    merge::{Merge, Merged},
};

/// One organised, attended event with two alarms, the ancestor every case
/// here is an edit of.
const BASE: &str = "BEGIN:VCALENDAR\r\n\
    VERSION:2.0\r\n\
    PRODID:-//Test//EN\r\n\
    BEGIN:VEVENT\r\n\
    UID:e1@example\r\n\
    DTSTAMP:20260101T000000Z\r\n\
    DTSTART:20260105T090000Z\r\n\
    SUMMARY:Standup\r\n\
    DESCRIPTION:The daily\r\n\
    LOCATION:Room A\r\n\
    ORGANIZER:mailto:chair@example.com\r\n\
    ATTENDEE;PARTSTAT=NEEDS-ACTION;CN=Ada:mailto:ada@example.com\r\n\
    ATTENDEE;PARTSTAT=NEEDS-ACTION;CN=Bob:mailto:bob@example.com\r\n\
    BEGIN:VALARM\r\n\
    ACTION:DISPLAY\r\n\
    TRIGGER:-PT15M\r\n\
    END:VALARM\r\n\
    BEGIN:VALARM\r\n\
    ACTION:AUDIO\r\n\
    TRIGGER:-PT5M\r\n\
    END:VALARM\r\n\
    END:VEVENT\r\n\
    END:VCALENDAR\r\n";

/// A series with one overriding instance, for the recurrence case.
const SERIES: &str = "BEGIN:VCALENDAR\r\n\
    VERSION:2.0\r\n\
    PRODID:-//Test//EN\r\n\
    BEGIN:VEVENT\r\n\
    UID:e1@example\r\n\
    DTSTAMP:20260101T000000Z\r\n\
    DTSTART:20260105T090000Z\r\n\
    SUMMARY:Standup\r\n\
    RRULE:FREQ=DAILY\r\n\
    END:VEVENT\r\n\
    BEGIN:VEVENT\r\n\
    UID:e1@example\r\n\
    RECURRENCE-ID:20260107T090000Z\r\n\
    DTSTAMP:20260101T000000Z\r\n\
    DTSTART:20260107T100000Z\r\n\
    SUMMARY:Standup\r\n\
    END:VEVENT\r\n\
    END:VCALENDAR\r\n";

/// One property a merge can put to a reader: the ancestor's content line,
/// how the value is spelled in the calendar, and the TOML key contested.
#[derive(Debug)]
struct Contested {
    /// The ancestor's content line, replaced whole by each side.
    from: &'static str,
    /// The line a side writes instead, `{}` standing in for its value.
    to: &'static str,
    /// The TOML key the collision writes once per side.
    key: &'static str,
    /// The block header the key must stay inside, for a property nested in a
    /// component of its own.
    header: Option<&'static str>,
}

/// The properties whose collision the projection can address, one per shape:
/// a bare text key, a date, and a key inside a nested component.
const CONTESTED: &[Contested] = &[
    Contested {
        from: "SUMMARY:Standup",
        to: "SUMMARY:{}",
        key: "summary",
        header: None,
    },
    Contested {
        from: "DESCRIPTION:The daily",
        to: "DESCRIPTION:{}",
        key: "description",
        header: None,
    },
    Contested {
        from: "LOCATION:Room A",
        to: "LOCATION:{}",
        key: "location",
        header: None,
    },
    Contested {
        from: "ACTION:AUDIO",
        to: "ACTION:{}",
        key: "action",
        header: Some("[[event.alarm]]"),
    },
];

/// The side of a merge that replaces the contested line with its own value.
fn side(spec: &Contested, value: &str) -> String {
    BASE.replace(spec.from, &spec.to.replace("{}", value))
}

/// Merge two sides of one contested property against the shared ancestor.
fn merged(spec: &Contested, local: &str, remote: &str) -> Merged {
    Merge {
        base: BASE,
        local: &side(spec, local),
        remote: &side(spec, remote),
        speaks_for: None,
    }
    .project()
    .unwrap()
}

/// A value distinct enough to be told apart in a calendar and in a document,
/// and plain enough to need no escaping in either.
fn value() -> impl Strategy<Value = String> {
    "[A-Z]{4,8}".prop_map(String::from)
}

prop_compose! {
    /// One collision: a property, and two values for it that differ from
    /// each other and from the ancestor.
    fn collision()(
        which in 0..CONTESTED.len(),
        local in value(),
        remote in value(),
    ) -> (&'static Contested, String, String) {
        (&CONTESTED[which], format!("L{local}"), format!("R{remote}"))
    }
}

/// The document lines that keep one side of a collision and drop the other.
fn keeping(toml: &str, dropped: &str) -> String {
    toml.lines()
        .filter(|line| !line.ends_with(dropped))
        .map(|line| format!("{line}\n"))
        .collect()
}

/// The lines of every block a header opens, up to the next header.
fn blocks<'t>(toml: &'t str, header: &str) -> Vec<Vec<&'t str>> {
    let mut blocks = Vec::new();
    let mut lines = toml.lines().peekable();

    while let Some(line) = lines.next() {
        if line != header {
            continue;
        }

        let mut block = Vec::new();

        while lines.peek().is_some_and(|line| !line.starts_with('[')) {
            block.push(lines.next().unwrap());
        }

        blocks.push(block);
    }

    blocks
}

/// How many times each key is written in one block, comments left out.
fn counts<'t>(block: &[&'t str]) -> Vec<(&'t str, usize)> {
    let mut counts: Vec<(&str, usize)> = Vec::new();

    for line in block.iter().filter(|line| !line.starts_with('#')) {
        let Some((key, _)) = line.split_once('=') else {
            continue;
        };

        let key = key.trim();
        match counts.iter_mut().find(|(held, _)| *held == key) {
            Some((_, count)) => *count += 1,
            None => counts.push((key, 1)),
        }
    }

    counts
}

/// The document's header comment as one unwrapped line, so a note can be
/// looked for without minding where it happens to have been folded.
fn notes(toml: &str) -> String {
    toml.lines()
        .take_while(|line| line.starts_with('#'))
        .map(|line| line.trim_start_matches('#').trim())
        .collect::<Vec<_>>()
        .join(" ")
}

proptest! {
    /// A document holding an addressable collision does not parse, and the
    /// refusal is reported as something left undecided rather than as a
    /// syntax error. The two live lines and the commented ancestor are all
    /// there, so the reader can see what is being asked.
    #[test]
    fn an_undecided_document_is_refused_as_undecided((spec, local, remote) in collision()) {
        let merged = merged(spec, &local, &remote);

        let left = format!("{} = \"{}\" # local", spec.key, local);
        let right = format!("{} = \"{}\" # remote", spec.key, remote);

        prop_assert!(merged.toml.contains("# conflict, keep one"));
        prop_assert!(merged.toml.contains(&left), "no local line: {}", merged.toml);
        prop_assert!(merged.toml.contains(&right), "no remote line: {}", merged.toml);

        match merged.apply(&merged.toml) {
            Err(TcalError::Undecided(_)) => {}
            other => prop_assert!(false, "not refused as undecided: {:?}", other.map(|_| ())),
        }
    }

    /// Deleting the lines of one side decides the collision for the other,
    /// in both directions. A renderer that always found the same line would
    /// pass a one-sided test, so both are asked for.
    #[test]
    fn keeping_one_side_yields_that_side((spec, local, remote) in collision()) {
        let merged = merged(spec, &local, &remote);

        let kept_local = merged.apply(&keeping(&merged.toml, "# remote")).unwrap();
        prop_assert!(kept_local.contains(&local), "{}", kept_local);
        prop_assert!(!kept_local.contains(&remote), "{}", kept_local);

        let kept_remote = merged.apply(&keeping(&merged.toml, "# local")).unwrap();
        prop_assert!(kept_remote.contains(&remote), "{}", kept_remote);
        prop_assert!(!kept_remote.contains(&local), "{}", kept_remote);
    }

    /// Replacing every line of a collision with a value of the reader's own
    /// yields that value, neither side's.
    #[test]
    fn replacing_the_lines_yields_ones_own_value((spec, local, remote) in collision()) {
        let merged = merged(spec, &local, &remote);

        let mine: String = merged
            .toml
            .lines()
            .filter(|line| !line.ends_with("# remote"))
            .map(|line| match line.ends_with("# local") {
                true => format!("{} = \"DECIDED\"\n", spec.key),
                false => format!("{line}\n"),
            })
            .collect();

        let out = merged.apply(&mine).unwrap();

        prop_assert!(out.contains("DECIDED"), "{}", out);
        prop_assert!(!out.contains(&local), "{}", out);
        prop_assert!(!out.contains(&remote), "{}", out);
    }

    /// The commented ancestor is a comment and nothing more: deleting it
    /// decides nothing and changes nothing.
    #[test]
    fn the_commented_ancestor_decides_nothing((spec, local, remote) in collision()) {
        let merged = merged(spec, &local, &remote);

        let with = keeping(&merged.toml, "# remote");
        let without = keeping(&with, "# base");

        prop_assert_ne!(&with, &without);
        prop_assert_eq!(merged.apply(&with).unwrap(), merged.apply(&without).unwrap());
    }

    /// A collision inside a nested component stays inside the one block that
    /// component projects to. Repeating the array-of-tables header instead
    /// is legal TOML: it would make a second alarm rather than a parse
    /// error, and the forcing would vanish exactly where the value is most
    /// complex.
    #[test]
    fn a_nested_collision_never_repeats_its_block_header((spec, local, remote) in collision()) {
        let Some(header) = spec.header else {
            return Ok(());
        };

        let merged = merged(spec, &local, &remote);

        prop_assert_eq!(
            merged.toml.lines().filter(|line| *line == header).count(),
            2,
            "{} is not written once per alarm",
            header,
        );

        // Exactly one of the two alarms carries the contest: the contested
        // key is written once per side there, every other key once, and the
        // untouched alarm holds no duplicate at all.
        let blocks = blocks(&merged.toml, header);
        let contesting = blocks
            .iter()
            .filter(|block| block.iter().any(|line| line.ends_with("# local")))
            .count();
        prop_assert_eq!(contesting, 1);

        for block in &blocks {
            let contests = block.iter().any(|line| line.ends_with("# local"));

            for (key, count) in counts(block) {
                let expected = usize::from(contests && key == spec.key) + 1;
                prop_assert_eq!(count, expected, "{}", key);
            }
        }
    }
}

/// A contested alarm is written as duplicate keys inside its own block, and
/// the two alarms of the ancestor stay two alarms.
#[test]
fn a_contested_alarm_stays_one_alarm() {
    let local = BASE.replace("TRIGGER:-PT5M", "TRIGGER:-PT6M");
    let remote = BASE.replace("TRIGGER:-PT5M", "TRIGGER:-PT7M");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert_eq!(merged.toml.matches("[[event.alarm]]").count(), 2);
    assert_eq!(merged.toml.matches("action = \"AUDIO\"").count(), 1);
    assert!(merged.toml.contains("trigger.min = 6 # local"));
    assert!(merged.toml.contains("trigger.min = 7 # remote"));
    assert!(merged.apply(&merged.toml).is_err());

    let decided = keeping(&merged.toml, "# remote");
    assert!(merged.apply(&decided).unwrap().contains("TRIGGER:-PT6M"));
}

/// A contested attendee is written as duplicate keys inside the one table
/// that attendee projects to, and the two attendees stay two attendees.
#[test]
fn a_contested_attendee_stays_one_attendee() {
    let local = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Bob", "PARTSTAT=ACCEPTED;CN=Bob");
    let remote = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Bob", "PARTSTAT=DECLINED;CN=Bob");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert_eq!(merged.toml.matches("[[event.attendee]]").count(), 2);
    assert!(merged.toml.contains("status = \"ACCEPTED\" # local"));
    assert!(merged.toml.contains("status = \"DECLINED\" # remote"));

    // Ada is untouched, so her table is written once and holds no contest.
    assert_eq!(merged.toml.matches("display-name = \"Ada\"").count(), 1);

    // Bob's other keys agree, so they are written once and only the key in
    // dispute is duplicated, which is the one the refusal names.
    assert_eq!(merged.toml.matches("display-name = \"Bob\"").count(), 1);
    match merged.apply(&merged.toml) {
        Err(TcalError::Undecided(key)) => assert_eq!(key, "status"),
        other => panic!("not refused as undecided: {:?}", other.map(|_| ())),
    }

    let kept_remote = merged.apply(&keeping(&merged.toml, "# local")).unwrap();
    assert!(kept_remote.contains("PARTSTAT=DECLINED"), "{kept_remote}");
    assert!(!kept_remote.contains("PARTSTAT=ACCEPTED"), "{kept_remote}");

    // The resolution keeps one attendee for Bob, not two: a silently
    // duplicated attendee changes who is invited.
    assert_eq!(kept_remote.matches("mailto:bob@example.com").count(), 1);
}

/// Two sides changing a different key of one attendee do not collide at all:
/// the addressing is per property, so both changes survive in the one table
/// the attendee projects to.
#[test]
fn two_sides_changing_different_keys_of_one_attendee_agree() {
    let local = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Bob", "PARTSTAT=ACCEPTED;CN=Bob");
    let remote = BASE.replace(
        "PARTSTAT=NEEDS-ACTION;CN=Bob",
        "PARTSTAT=NEEDS-ACTION;ROLE=CHAIR;CN=Bob",
    );

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert_eq!(merged.toml.matches("[[event.attendee]]").count(), 2);
    assert!(!merged.toml.contains("# conflict"), "{}", merged.toml);
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A collision the projection spells the same way on both sides is a header
/// comment: the difference sits in a parameter it never shows, so offering it
/// as a choice would ask the reader to pick between two identical lines.
#[test]
fn a_collision_the_projection_cannot_tell_apart_is_a_comment() {
    let local = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Ada", "RSVP=TRUE;CN=Ada");
    let remote = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Ada", "RSVP=FALSE;CN=Ada");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert_eq!(merged.toml.matches("display-name = \"Ada\"").count(), 1);
    assert!(!merged.toml.contains("# conflict"), "{}", merged.toml);
    assert!(notes(&merged.toml).contains("the local value was kept"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A value the projection writes as several lines is contested whole: every
/// line the two sides spell differently is written once per side, deleting
/// one of them still refuses, and deleting the side whole yields that side's
/// date and zone together rather than half of each.
#[test]
fn a_multi_line_value_is_contested_whole() {
    let base = BASE.replace(
        "DTSTART:20260105T090000Z",
        "DTSTART;TZID=Europe/Paris:20260105T090000",
    );
    let local = base.replace(
        "DTSTART;TZID=Europe/Paris:20260105T090000",
        "DTSTART;TZID=Europe/Paris:20260105T100000",
    );
    let remote = base.replace(
        "DTSTART;TZID=Europe/Paris:20260105T090000",
        "DTSTART;TZID=Europe/Berlin:20260105T110000",
    );

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    // Both lines of the value differ, so both are contested, and the header
    // asks for a side rather than a line.
    assert!(merged.toml.contains("# conflict, keep one side"));
    assert!(
        merged
            .toml
            .contains("date-start = 2026-01-05T10:00:00 # local")
    );
    assert!(
        merged
            .toml
            .contains("date-start-tz = \"Europe/Paris\" # local")
    );
    assert!(
        merged
            .toml
            .contains("date-start = 2026-01-05T11:00:00 # remote")
    );
    assert!(
        merged
            .toml
            .contains("date-start-tz = \"Europe/Berlin\" # remote")
    );

    // Deleting one line of a side leaves the other duplicated, so the
    // document still refuses rather than splicing the two sides together.
    let half = merged
        .toml
        .replace("date-start = 2026-01-05T11:00:00 # remote\n", "");
    assert!(merged.apply(&half).is_err());

    let kept_local = merged.apply(&keeping(&merged.toml, "# remote")).unwrap();
    assert!(
        kept_local.contains("DTSTART;TZID=Europe/Paris:20260105T100000"),
        "{kept_local}",
    );
}

/// A collision the merge already settled is a header comment, not duplicate
/// keys: a removal against an update has nothing to choose, and the document
/// applies as it stands.
#[test]
fn a_removal_against_an_update_is_a_comment() {
    let local = BASE.replace("SUMMARY:Standup\r\n", "");
    let remote = BASE.replace("SUMMARY:Standup", "SUMMARY:Team standup");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert!(merged.ical.contains("SUMMARY:Team standup"));
    assert!(notes(&merged.toml).contains("removed on local and updated on remote"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A rule changed on one side against an instance changed on the other is
/// settled by keeping both, so there is nothing to choose and the pair is
/// said in a comment.
#[test]
fn a_rule_against_an_instance_is_a_comment() {
    let local = SERIES.replace("RRULE:FREQ=DAILY", "RRULE:FREQ=WEEKLY");
    let remote = SERIES.replace("DTSTART:20260107T100000Z", "DTSTART:20260107T110000Z");

    let merged = Merge {
        base: SERIES,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert!(merged.ical.contains("RRULE:FREQ=WEEKLY"));
    assert!(merged.ical.contains("DTSTART:20260107T110000Z"));
    assert!(notes(&merged.toml).contains("one is a series"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A change refused for want of organiser authority is a header comment: the
/// merge settled it by refusing, so there is nothing to put to the reader.
/// vCard has no organiser, so this law has no sibling in tcard.
#[test]
fn a_refusal_for_want_of_authority_is_a_comment() {
    let local = BASE.replace("DTSTART:20260105T090000Z", "DTSTART:20260105T100000Z");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: BASE,
        speaks_for: Some("ada@example.com"),
    }
    .project()
    .unwrap();

    assert!(merged.ical.contains("DTSTART:20260105T090000Z"));
    assert!(!merged.ical.contains("100000Z"));
    assert!(notes(&merged.toml).contains("organiser"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A collision the projection cannot address keeps the local value, says so
/// in the header, and leaves the document appliable: there is no key to
/// write twice, so there is nothing to force.
#[test]
fn an_unprojectable_collision_keeps_the_local_value_and_says_so() {
    let base = BASE.replace("SUMMARY:Standup", "SUMMARY:Standup\r\nX-FOO:one");
    let local = base.replace("X-FOO:one", "X-FOO:two");
    let remote = base.replace("X-FOO:one", "X-FOO:three");

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    assert!(merged.ical.contains("X-FOO:two"));
    assert!(!merged.ical.contains("X-FOO:three"));
    assert!(notes(&merged.toml).contains("the local value was kept"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// The refusal names a key the reader can find in the document, so that
/// being told what is undecided is help rather than a riddle.
#[test]
fn the_refusal_names_a_key_the_document_writes() {
    let local = BASE.replace("TRIGGER:-PT5M", "TRIGGER:-PT6M");
    let remote = BASE.replace("TRIGGER:-PT5M", "TRIGGER:-PT7M");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    let Err(TcalError::Undecided(key)) = merged.apply(&merged.toml) else {
        panic!("not refused as undecided");
    };

    // The document writes trigger.min, trigger.week and so on; it writes no
    // bare "week" anywhere, so naming one sends the reader looking for a key
    // that is not there.
    assert!(
        merged
            .toml
            .lines()
            .any(|line| line.starts_with(&format!("{key} ="))),
        "the document writes no {key:?} key",
    );
}

/// A collision on a multi-valued property is put to the reader like any
/// other, rather than being settled by keeping every item both sides wrote.
///
/// See findings/tcal-list-collision-silently-unioned.md.
#[test]
#[ignore = "fails: see findings/tcal-list-collision-silently-unioned.md"]
fn a_list_collision_is_put_to_the_reader() {
    let base = BASE.replace("SUMMARY:Standup", "SUMMARY:Standup\r\nCATEGORIES:a,b");
    let local = base.replace("CATEGORIES:a,b", "CATEGORIES:c,d");
    let remote = base.replace("CATEGORIES:a,b", "CATEGORIES:e,f");

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    // Today the merge writes CATEGORIES:e,f,c,d, a value neither side wrote,
    // and the document says nothing at all about it.
    assert!(
        merged.toml.contains("# conflict") || merged.toml.contains("categories"),
        "{}",
        merged.toml,
    );
    assert!(
        !merged.ical.contains("CATEGORIES:e,f,c,d"),
        "{}",
        merged.ical
    );
}

/// A collision on one attendee survives a removal of a different one: the
/// merge either puts it to the reader or says how it settled it, but never
/// drops one side's decision without a word.
///
/// See findings/tcal-a-removal-swallows-a-neighbours-collision.md.
#[test]
#[ignore = "fails: see findings/tcal-a-removal-swallows-a-neighbours-collision.md"]
fn a_removal_does_not_swallow_a_neighbours_collision() {
    // Local drops Ada and accepts as Bob; remote declines as Bob. Bob is
    // contested on both sides, and Ada has nothing to do with it.
    let local = BASE
        .replace(
            "ATTENDEE;PARTSTAT=NEEDS-ACTION;CN=Ada:mailto:ada@example.com\r\n",
            "",
        )
        .replace("PARTSTAT=NEEDS-ACTION;CN=Bob", "PARTSTAT=ACCEPTED;CN=Bob");
    let remote = BASE.replace("PARTSTAT=NEEDS-ACTION;CN=Bob", "PARTSTAT=DECLINED;CN=Bob");

    let merged = Merge {
        base: BASE,
        local: &local,
        remote: &remote,
        speaks_for: None,
    }
    .project()
    .unwrap();

    // Today the merge keeps ACCEPTED, drops DECLINED, and the document
    // carries neither a contest nor a note about it.
    assert!(
        merged.toml.contains("# conflict") || merged.toml.contains("# - "),
        "{}",
        merged.toml,
    );
}
