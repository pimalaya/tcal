//! # Content lines
//!
//! The grammar a projected value is read through and patched with: a line
//! split into its head and its value, the RFC 5545 section 3.3.11 escapes that
//! value carries, and the scheme a calendar address wears.
//!
//! The projection shows a modelled property's value and the few parameters it
//! has keys for. Rebuilding the line out of the document alone would therefore
//! drop every other parameter (`RSVP`, `SENT-BY`, `ALTREP`, `LANGUAGE`), so
//! the line is patched instead.

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

/// One content line, without its line ending.
pub struct Content<'a>(pub &'a str);

impl<'a> Content<'a> {
    /// The name and parameters of the line, its value excluded.
    pub fn head(&self) -> &'a str {
        &self.0[..colon(self.0)]
    }

    /// The value of the line, its name and parameters excluded.
    pub fn value(&self) -> &'a str {
        self.0.get(colon(self.0) + 1..).unwrap_or_default()
    }

    /// The value as one unescaped string, its commas kept literal.
    pub fn text(&self) -> String {
        unescape(self.value())
    }

    /// The value as its comma-separated items, each unescaped on its own.
    ///
    /// An escaped comma stays inside the item it belongs to.
    pub fn texts(&self) -> Vec<String> {
        let value = self.value();
        let mut out = Vec::new();
        let mut start = 0;
        let mut escaped = false;

        for (at, ch) in value.char_indices() {
            match ch {
                '\\' if !escaped => escaped = true,
                ',' if !escaped => {
                    out.push(unescape(&value[start..at]));
                    start = at + 1;
                }
                _ => escaped = false,
            }
        }

        out.push(unescape(&value[start..]));
        out
    }

    /// The line a fold-back writes, patched onto this one it came from.
    ///
    /// A parameter the projection shows is the document's, dropped where the
    /// document cleared it; every other one is this line's own and stays where
    /// it stood.
    pub fn rewritten(&self, line: &str, shown: &[&str]) -> String {
        let held = split(self.head(), ';');
        let written = split(Content(line).head(), ';');
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
        out.push_str(Content(line).value());
        out
    }
}

/// Escape an iCalendar text value per RFC 5545 section 3.3.11.
pub fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());

    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            ',' => out.push_str("\\,"),
            ';' => out.push_str("\\;"),
            '\n' => out.push_str("\\n"),
            _ => out.push(ch),
        }
    }

    out
}

/// Undo that escaping, the inverse of [`escape`].
///
/// What a calendar wrote as `\,` is a comma, and either spelling of an escaped
/// newline is one.
pub fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        match chars.next() {
            Some('n' | 'N') => out.push('\n'),
            Some(next) => out.push(next),
            None => out.push('\\'),
        }
    }

    out
}

/// Split on every `sep` outside a quoted parameter value.
pub fn split(text: &str, sep: char) -> Vec<&str> {
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

/// A calendar address without its `mailto:` scheme (any case), for display.
pub fn strip_mailto(value: &str) -> &str {
    match value.get(..7) {
        Some(scheme) if scheme.eq_ignore_ascii_case("mailto:") => &value[7..],
        _ => value,
    }
}

/// A calendar address with a scheme: a bare address gains `mailto:`.
pub fn ensure_mailto(value: &str) -> String {
    if value.contains(':') {
        value.to_owned()
    } else {
        format!("mailto:{value}")
    }
}

/// Where the colon ending a line's name and parameters sits.
///
/// A colon inside a quoted parameter value (`SENT-BY="mailto:s@x"`) does not
/// count.
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

/// Append one parameter to a line's head.
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

    use crate::template::patch::Content;

    #[test]
    fn a_quoted_parameter_holds_its_own_colon() {
        let line = Content("ORGANIZER;SENT-BY=\"mailto:s@x\":mailto:chair@example.com");

        assert_eq!(line.head(), "ORGANIZER;SENT-BY=\"mailto:s@x\"");
        assert_eq!(line.value(), "mailto:chair@example.com");
    }

    #[test]
    fn an_unshown_parameter_is_kept_where_it_stood() {
        let original =
            Content("ATTENDEE;RSVP=TRUE;PARTSTAT=NEEDS-ACTION;CUTYPE=INDIVIDUAL:mailto:a@x");

        assert_eq!(
            original.rewritten(
                "ATTENDEE;PARTSTAT=ACCEPTED:mailto:a@x",
                &["CN", "ROLE", "PARTSTAT"],
            ),
            "ATTENDEE;RSVP=TRUE;PARTSTAT=ACCEPTED;CUTYPE=INDIVIDUAL:mailto:a@x",
        );

        assert_eq!(
            Content("SUMMARY;LANGUAGE=en:a").rewritten("SUMMARY:b", &[]),
            "SUMMARY;LANGUAGE=en:b",
        );
    }

    #[test]
    fn a_shown_parameter_is_the_documents_to_drop() {
        assert_eq!(
            Content("ATTENDEE;RSVP=TRUE;PARTSTAT=NEEDS-ACTION:mailto:a@x")
                .rewritten("ATTENDEE:mailto:a@x", &["CN", "ROLE", "PARTSTAT"],),
            "ATTENDEE;RSVP=TRUE:mailto:a@x",
        );
        assert_eq!(
            Content("DTSTART;TZID=Europe/Paris:20260105T090000")
                .rewritten("DTSTART;VALUE=DATE:20260105", &["TZID", "VALUE"],),
            "DTSTART;VALUE=DATE:20260105",
        );
    }

    #[test]
    fn an_escaped_comma_stays_inside_its_item() {
        assert_eq!(Content("X:a,b").texts(), ["a".to_string(), "b".to_string()]);
        assert_eq!(Content("X:a\\,b").texts(), ["a,b".to_string()]);
        assert_eq!(Content("X:").texts(), ["".to_string()]);
    }
}
