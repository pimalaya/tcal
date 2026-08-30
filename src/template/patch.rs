//! # Patching a content line
//!
//! Taking apart the line a projected value came from, so folding the
//! document back rewrites only what the document writes.
//!
//! The projection shows a modeled property's value and the few parameters
//! it has keys for. Rebuilding the line out of the document alone would
//! therefore drop every other parameter (`RSVP`, `SENT-BY`, `ALTREP`,
//! `LANGUAGE`), so the line is patched instead.

use alloc::{borrow::ToOwned, string::String, vec::Vec};

/// The line a fold-back writes, patched onto the line its value came from.
///
/// A parameter the projection shows is the document's, dropped where the
/// document cleared it; every other one is the line's own and stays where it
/// stood. An original of `None` is a line the calendar does not hold yet.
pub fn rewritten(original: Option<&str>, line: &str, shown: &[&str]) -> String {
    let Some(original) = original else {
        return line.to_owned();
    };

    let held = split(head(original), ';');
    let written = split(head(line), ';');
    let mut out = String::from(written.first().copied().unwrap_or_default());

    for param in held.iter().skip(1) {
        match written.iter().skip(1).find(|mine| named(mine, param)) {
            Some(mine) => push(&mut out, mine),
            None if !shown.iter().any(|name| named_after(param, name)) => push(&mut out, param),
            None => {}
        }
    }

    for param in written.iter().skip(1) {
        if !held.iter().skip(1).any(|kept| named(kept, param)) {
            push(&mut out, param);
        }
    }

    out.push(':');
    out.push_str(value_of(line));
    out
}

/// The name and parameters of a content line, its value excluded.
pub(crate) fn head(line: &str) -> &str {
    &line[..colon(line)]
}

/// The value of a content line, its name and parameters excluded.
pub(crate) fn value_of(line: &str) -> &str {
    line.get(colon(line) + 1..).unwrap_or_default()
}

/// Where the colon ending a line's name and parameters sits.
///
/// A colon inside a quoted parameter value (`SENT-BY="mailto:s@x"`) does
/// not count.
fn colon(line: &str) -> usize {
    let mut quoted = false;

    for (at, ch) in line.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            ':' if !quoted => return at,
            _ => {}
        }
    }

    line.len()
}

/// Split on every `sep` outside a quoted parameter value.
pub(crate) fn split(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut quoted = false;
    let mut start = 0;

    for (at, ch) in text.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            _ if ch == sep && !quoted => {
                out.push(&text[start..at]);
                start = at + ch.len_utf8();
            }
            _ => {}
        }
    }

    out.push(&text[start..]);
    out
}

/// Append one parameter to a line's prefix.
fn push(out: &mut String, param: &str) {
    out.push(';');
    out.push_str(param);
}

/// Whether two parameters name the same thing.
///
/// iCalendar compares parameter names without regard to case (RFC 5545
/// section 3.2).
fn named(one: &str, other: &str) -> bool {
    named_after(one, name_of(other))
}

/// Whether a parameter carries the given name.
fn named_after(param: &str, name: &str) -> bool {
    name_of(param).eq_ignore_ascii_case(name)
}

/// The name a parameter carries, its value excluded.
fn name_of(param: &str) -> &str {
    param.split_once('=').map_or(param, |(name, _)| name)
}

#[cfg(test)]
mod tests {
    use alloc::string::ToString;

    #[test]
    fn a_quoted_parameter_holds_its_own_colon() {
        let line = "ORGANIZER;SENT-BY=\"mailto:s@x\":mailto:chair@example.com";

        assert_eq!(super::head(line), "ORGANIZER;SENT-BY=\"mailto:s@x\"");
        assert_eq!(super::value_of(line), "mailto:chair@example.com");
    }

    #[test]
    fn an_unshown_parameter_is_kept_where_it_stood() {
        let original = "ATTENDEE;RSVP=TRUE;PARTSTAT=NEEDS-ACTION;CUTYPE=INDIVIDUAL:mailto:a@x";

        assert_eq!(
            super::rewritten(
                Some(original),
                "ATTENDEE;PARTSTAT=ACCEPTED:mailto:a@x",
                &["CN", "ROLE", "PARTSTAT"],
            ),
            "ATTENDEE;RSVP=TRUE;PARTSTAT=ACCEPTED;CUTYPE=INDIVIDUAL:mailto:a@x",
        );

        assert_eq!(
            super::rewritten(Some("SUMMARY;LANGUAGE=en:a"), "SUMMARY:b", &[]),
            "SUMMARY;LANGUAGE=en:b",
        );
    }

    #[test]
    fn a_shown_parameter_is_the_documents_to_drop() {
        assert_eq!(
            super::rewritten(
                Some("ATTENDEE;RSVP=TRUE;PARTSTAT=NEEDS-ACTION:mailto:a@x"),
                "ATTENDEE:mailto:a@x",
                &["CN", "ROLE", "PARTSTAT"],
            ),
            "ATTENDEE;RSVP=TRUE:mailto:a@x",
        );
        assert_eq!(
            super::rewritten(
                Some("DTSTART;TZID=Europe/Paris:20260105T090000"),
                "DTSTART;VALUE=DATE:20260105",
                &["TZID", "VALUE"],
            ),
            "DTSTART;VALUE=DATE:20260105",
        );
    }

    #[test]
    fn a_new_line_is_written_as_the_document_wrote_it() {
        assert_eq!(
            super::rewritten(None, "SUMMARY:a", &[]),
            "SUMMARY:a".to_string(),
        );
    }
}
