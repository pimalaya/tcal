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
//! What the merge settled on its own is said in the header instead, and the
//! laws below cover that too: a removal against an update, a part the
//! projection does not show, a refusal for want of authority, and a list both
//! sides edited, whose items are all kept.

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
    /// The block the key must stay inside, for a property nested in a
    /// component of its own: the document header that opens one, and the
    /// calendar line that makes one.
    header: Option<(&'static str, &'static str)>,
}

/// The properties whose collision the projection can address, one per shape:
/// a bare key, a key inside a nested component, and a key inside the table
/// an attendee projects to.
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
        header: Some(("[[event.alarm]]", "BEGIN:VALARM")),
    },
    Contested {
        from: "PARTSTAT=NEEDS-ACTION;CN=Bob",
        to: "PARTSTAT={};CN=Bob",
        key: "status",
        header: Some(("[[event.attendee]]", "ATTENDEE")),
    },
];

/// The attendee a side may drop beside the property it contests, which moves
/// every later attendee out of the position the report counted it at.
const NEIGHBOUR: &str = "ATTENDEE;PARTSTAT=NEEDS-ACTION;CN=Ada:mailto:ada@example.com\r\n";

/// The side of a merge that replaces the contested line with its own value,
/// having dropped the neighbouring attendee first where it removes one.
fn side(spec: &Contested, value: &str, removes: bool) -> String {
    let base = if removes {
        BASE.replace(NEIGHBOUR, "")
    } else {
        BASE.to_string()
    };

    base.replace(spec.from, &spec.to.replace("{}", value))
}

/// Merge two sides of one contested property against the shared ancestor,
/// the local side removing the neighbouring attendee where it does.
fn merged(spec: &Contested, local: &str, remote: &str, removes: bool) -> Merged {
    let merged = Merge {
        base: BASE,
        local: &side(spec, local, removes),
        remote: &side(spec, remote, false),
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);
    merged
}

/// The preamble announces exactly the contests the document writes below it.
///
/// A document announcing one and holding none sends the reader to decide
/// something it never shows them, then parses, applies and takes one side
/// without a word, which is the whole forcing gone.
fn announces_what_it_holds(merged: &Merged) {
    assert_eq!(
        announced(&merged.toml),
        merged.toml.matches("# conflict, keep one").count(),
        "{}",
        merged.toml,
    );
}

/// The number of conflicts the preamble announces, none where it announces
/// nothing.
fn announced(toml: &str) -> usize {
    let header = notes(toml);
    let Some(at) = header.find(" conflict") else {
        return 0;
    };

    header[..at]
        .rsplit(' ')
        .next()
        .and_then(|count| count.parse().ok())
        .unwrap_or_default()
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
        removes in proptest::bool::ANY,
    ) -> (&'static Contested, String, String, bool) {
        (&CONTESTED[which], format!("L{local}"), format!("R{remote}"), removes)
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
    fn an_undecided_document_is_refused_as_undecided((spec, local, remote, removes) in collision()) {
        let merged = merged(spec, &local, &remote, removes);

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
    fn keeping_one_side_yields_that_side((spec, local, remote, removes) in collision()) {
        let merged = merged(spec, &local, &remote, removes);

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
    fn replacing_the_lines_yields_ones_own_value((spec, local, remote, removes) in collision()) {
        let merged = merged(spec, &local, &remote, removes);

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
    fn the_commented_ancestor_decides_nothing((spec, local, remote, removes) in collision()) {
        let merged = merged(spec, &local, &remote, removes);

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
    fn a_nested_collision_never_repeats_its_block_header((spec, local, remote, removes) in collision()) {
        let Some((header, source)) = spec.header else {
            return Ok(());
        };

        let merged = merged(spec, &local, &remote, removes);

        // One block per surviving component, however many a side removed.
        prop_assert_eq!(
            merged.toml.lines().filter(|line| *line == header).count(),
            merged.ical.matches(source).count(),
            "{} is not written once per component",
            header,
        );

        // Exactly one block carries the contest: the contested key is
        // written once per side there, every other key once, and an
        // untouched block holds no duplicate at all.
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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

    assert!(merged.ical.contains("RRULE:FREQ=WEEKLY"));
    assert!(merged.ical.contains("DTSTART:20260107T110000Z"));
    assert!(notes(&merged.toml).contains("one is a series"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

#[test]
fn an_unprojectable_collision_keeps_the_local_value_and_says_so() {
    let base = BASE.replace("SUMMARY:Standup", "SUMMARY:Standup\r\nX-FOO:one");
    let local = base.replace("X-FOO:one", "X-FOO:two");
    let remote = base.replace("X-FOO:one", "X-FOO:three");

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

    assert!(merged.ical.contains("X-FOO:two"));
    assert!(!merged.ical.contains("X-FOO:three"));
    assert!(notes(&merged.toml).contains("the local value was kept"));
    assert!(!merged.toml.contains("# conflict"));
    assert!(merged.apply(&merged.toml).is_ok());
}

/// A note longer than the column the document is written to is folded over
/// two comment lines, the second indented under the first line's text, so the
/// header keeps the width everything below it keeps.
#[test]
fn a_long_note_wraps_under_itself() {
    let base = BASE.replace("SUMMARY:Standup", "SUMMARY:Standup\r\nX-FOO:one");
    let local = base.replace("X-FOO:one", "X-FOO:two");
    let remote = base.replace("X-FOO:one", "X-FOO:three");

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

    let header: Vec<&str> = merged
        .toml
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect();

    assert!(header.iter().all(|line| line.len() <= 68), "{header:#?}");
    assert!(
        header.iter().any(|line| line.starts_with("#   ")),
        "{header:#?}",
    );
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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

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

/// Both sides adding to a multi-valued property keeps every item, which is
/// right for a value RFC 5545 gives no order to, and the header says so, so
/// the union is something the reader reviews rather than something that
/// happened to them.
#[test]
fn a_list_union_is_said_in_the_header() {
    let base = BASE.replace("SUMMARY:Standup", "SUMMARY:Standup\r\nCATEGORIES:a,b");
    let local = base.replace("CATEGORIES:a,b", "CATEGORIES:c,d");
    let remote = base.replace("CATEGORIES:a,b", "CATEGORIES:e,f");

    let merged = Merge {
        base: &base,
        local: &local,
        remote: &remote,
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

    assert!(
        notes(&merged.toml).contains(
            "event 1 / categories: both sides changed its list; the items of both were kept."
        ),
        "{}",
        merged.toml,
    );
    assert!(!merged.toml.contains("# conflict"), "offered as a choice");

    // Nothing is left to choose, so the document applies as it stands, with
    // the whole list on the one key the reader can edit. The local items lead,
    // the merged calendar being built from the local side's bytes.
    assert!(
        merged.toml.contains(r#"categories = ["c", "d", "e", "f"]"#),
        "{}",
        merged.toml,
    );

    let out = merged.apply(&merged.toml).unwrap();
    assert!(out.contains("CATEGORIES:c,d,e,f"), "{out}");
}

/// A collision on one attendee survives a removal of a different one: the
/// removal moves the surviving attendee out of the position the report
/// counted it at, and the contest still lands on the table it wrote.
#[test]
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
    }
    .project()
    .unwrap();

    announces_what_it_holds(&merged);

    // Ada is gone, so Bob's table is the only one, and his answer is
    // contested in it rather than silently taken from the local side.
    assert_eq!(merged.toml.matches("[[event.attendee]]").count(), 1);
    assert!(merged.toml.contains("status = \"ACCEPTED\" # local"));
    assert!(merged.toml.contains("status = \"DECLINED\" # remote"));

    match merged.apply(&merged.toml) {
        Err(TcalError::Undecided(key)) => assert_eq!(key, "status"),
        other => panic!("not refused as undecided: {:?}", other.map(|_| ())),
    }

    let kept_remote = merged.apply(&keeping(&merged.toml, "# local")).unwrap();
    assert!(kept_remote.contains("PARTSTAT=DECLINED"), "{kept_remote}");
    assert!(!kept_remote.contains("ada@example.com"), "{kept_remote}");
}
