//! A format-preserving iCalendar editor, the `toml_edit` analog for
//! iCalendar.
//!
//! calcard is a normalizing reader/writer: re-serializing churns line
//! folding, parameter casing and property order even where nothing
//! changed. This module instead keeps every content line's original
//! bytes and re-renders only the lines a caller mutates, so editing one
//! property yields a minimal diff.
//!
//! iCalendar is line-oriented (`NAME;PARAMS:VALUE`) with a single
//! wrinkle, line folding, so the layer is small. It is deliberately
//! calcard-independent (std only) and could move to its own crate later.
//!
//! The core invariant is round-trip identity: `parse(s).to_string() == s`
//! for any input.

use std::fmt;

/// The longest octet length of a physical line before folding kicks in,
/// per RFC 5545 section 3.1; mirrors calcard's writer.
const MAX_LINE_OCTETS: usize = 75;

/// A parsed iCalendar stream as a format-preserving tree.
pub struct Calendar {
    items: Vec<Item>,
}

/// Parse an iCalendar stream into a format-preserving tree. Infallible:
/// anything unrecognized is kept verbatim so output can round-trip.
pub fn parse(src: &str) -> Calendar {
    let logicals = unfold(src);

    let mut items = Vec::new();
    let mut cursor = 0;

    while cursor < logicals.len() {
        let (mut block, stop) = parse_block(&logicals, &mut cursor);
        items.append(&mut block);

        // A stray END with no open component is kept as-is, not dropped.
        if let Stop::End(end_raw) = stop {
            items.push(Item::Raw(end_raw));
        }
    }

    Calendar { items }
}

impl Calendar {
    /// The first component of the given type, searched depth-first and
    /// case-insensitively (`cal.component_mut("VEVENT")`).
    pub fn component_mut(&mut self, ty: &str) -> Option<&mut Component> {
        find_component_mut(&mut self.items, ty)
    }
}

impl fmt::Display for Calendar {
    /// Concatenate every node's raw bytes; only mutated or inserted
    /// properties were re-rendered, everything else is original.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for item in &self.items {
            item.fmt(f)?;
        }

        Ok(())
    }
}

/// A `BEGIN`/`END` component (`VEVENT`, `VALARM`, ...) and its contents.
pub struct Component {
    name: String,
    begin_raw: String,
    items: Vec<Item>,
    end_raw: String,
}

impl Component {
    /// The first descendant component of the given type, searched
    /// depth-first and case-insensitively.
    pub fn component_mut(&mut self, ty: &str) -> Option<&mut Component> {
        find_component_mut(&mut self.items, ty)
    }

    /// The logical content lines of this component's own direct
    /// properties matching `name` (no enclosing component is searched).
    pub fn get_all(&self, name: &str) -> Vec<&str> {
        let upper = name.to_uppercase();

        self.items
            .iter()
            .filter_map(|item| match item {
                Item::Property(property) if property.name == upper => {
                    Some(property.logical.as_str())
                }
                _ => None,
            })
            .collect()
    }

    /// Make this component's direct properties named `name` exactly equal
    /// `lines` (full content lines without an end of line, e.g.
    /// `"SUMMARY:Team lunch"`), with a minimal diff.
    ///
    /// Existing properties are reused in order: where the desired line
    /// already matches, the original bytes are left untouched; otherwise
    /// the line is re-rendered. Surplus properties are removed and missing
    /// ones inserted after the last direct property, before any nested
    /// component. `lines == []` removes every matching property.
    pub fn set_all(&mut self, name: &str, lines: &[String]) {
        let upper = name.to_uppercase();
        let eol = eol_of(&self.begin_raw).to_owned();

        let positions: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| match item {
                Item::Property(property) if property.name == upper => Some(index),
                _ => None,
            })
            .collect();

        // Reuse existing slots positionally, re-rendering only on change.
        let reuse = positions.len().min(lines.len());
        for slot in 0..reuse {
            if let Item::Property(property) = &mut self.items[positions[slot]]
                && property.logical != lines[slot]
            {
                property.logical = lines[slot].clone();
                property.raw = render(&lines[slot], &eol);
            }
        }

        if lines.len() < positions.len() {
            // Drop surplus from the back so earlier indices stay valid.
            for slot in (lines.len()..positions.len()).rev() {
                self.items.remove(positions[slot]);
            }
        } else if lines.len() > positions.len() {
            let at = self.insertion_point();
            let extras = lines[positions.len()..].iter().map(|line| {
                Item::Property(Property {
                    name: upper.clone(),
                    logical: line.clone(),
                    raw: render(line, &eol),
                })
            });

            let tail = self.items.split_off(at);
            self.items.extend(extras);
            self.items.extend(tail);
        }
    }

    /// Remove every direct property matching `name`.
    pub fn remove(&mut self, name: &str) {
        self.set_all(name, &[]);
    }

    /// Where a new property should land: after the last direct property,
    /// else before the first nested component, else at the end (which
    /// sits just before `END`, kept separately in `end_raw`).
    fn insertion_point(&self) -> usize {
        if let Some(last) = self
            .items
            .iter()
            .rposition(|item| matches!(item, Item::Property(_)))
        {
            return last + 1;
        }

        self.items
            .iter()
            .position(|item| matches!(item, Item::Component(_)))
            .unwrap_or(self.items.len())
    }
}

/// One node of a [`Calendar`]: a property, a nested component, or an
/// unrecognized line (blank line, junk) kept verbatim.
enum Item {
    Property(Property),
    Component(Component),
    Raw(String),
}

impl Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Property(property) => f.write_str(&property.raw),
            Item::Raw(raw) => f.write_str(raw),
            Item::Component(component) => {
                f.write_str(&component.begin_raw)?;
                for item in &component.items {
                    item.fmt(f)?;
                }
                f.write_str(&component.end_raw)
            }
        }
    }
}

/// A single content line (`NAME;PARAMS:VALUE`): unfolded for matching but
/// kept byte-for-byte (folding and end of line included) for output.
struct Property {
    name: String,
    logical: String,
    raw: String,
}

/// A logical (unfolded) line: its joined content and the exact original
/// bytes of every physical line that made it up.
struct Logical {
    content: String,
    raw: String,
}

/// Why [`parse_block`] returned: it consumed a closing `END` (whose raw
/// bytes it hands back) or reached the end of input.
enum Stop {
    End(String),
    Eof,
}

/// Parse items until the matching `END` or the end of input. `BEGIN`
/// recurses, so an `END` closes the innermost open component.
fn parse_block(logicals: &[Logical], cursor: &mut usize) -> (Vec<Item>, Stop) {
    let mut items = Vec::new();

    while *cursor < logicals.len() {
        let logical = &logicals[*cursor];

        if end_name(&logical.content).is_some() {
            let end_raw = logical.raw.clone();
            *cursor += 1;
            return (items, Stop::End(end_raw));
        }

        if let Some(name) = begin_name(&logical.content) {
            let begin_raw = logical.raw.clone();
            *cursor += 1;

            let (inner, stop) = parse_block(logicals, cursor);
            let end_raw = match stop {
                Stop::End(raw) => raw,
                Stop::Eof => String::new(),
            };

            items.push(Item::Component(Component {
                name,
                begin_raw,
                items: inner,
                end_raw,
            }));
            continue;
        }

        let item = match property_name(&logical.content) {
            Some(name) => Item::Property(Property {
                name,
                logical: logical.content.clone(),
                raw: logical.raw.clone(),
            }),
            None => Item::Raw(logical.raw.clone()),
        };
        items.push(item);
        *cursor += 1;
    }

    (items, Stop::Eof)
}

/// The first component of the given type within `items`, depth-first
/// (pre-order) and case-insensitive.
fn find_component_mut<'a>(items: &'a mut [Item], ty: &str) -> Option<&'a mut Component> {
    for item in items.iter_mut() {
        if let Item::Component(component) = item {
            if component.name.eq_ignore_ascii_case(ty) {
                return Some(component);
            }
            if let Some(found) = find_component_mut(&mut component.items, ty) {
                return Some(found);
            }
        }
    }

    None
}

/// Split `src` into logical lines, joining folded continuations (a
/// physical line starting with a space or tab) onto the previous line
/// while recording the exact original bytes.
fn unfold(src: &str) -> Vec<Logical> {
    let mut logicals: Vec<Logical> = Vec::new();

    for (content, raw) in physical_lines(src) {
        let is_continuation = content.starts_with(' ') || content.starts_with('\t');

        if is_continuation && let Some(last) = logicals.last_mut() {
            // Unfolding drops the CRLF and exactly one leading space.
            last.content.push_str(&content[1..]);
            last.raw.push_str(raw);
            continue;
        }

        logicals.push(Logical {
            content: content.to_owned(),
            raw: raw.to_owned(),
        });
    }

    logicals
}

/// Split `src` into physical lines, yielding for each its content (no end
/// of line) and its raw bytes (the end of line, when present, included).
fn physical_lines(src: &str) -> Vec<(&str, &str)> {
    let mut lines = Vec::new();
    let bytes = src.as_bytes();
    let mut start = 0;

    for (index, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }

        let raw = &src[start..=index];
        let content_end = if index > start && bytes[index - 1] == b'\r' {
            index - 1
        } else {
            index
        };
        lines.push((&src[start..content_end], raw));
        start = index + 1;
    }

    if start < bytes.len() {
        lines.push((&src[start..], &src[start..]));
    }

    lines
}

/// Render a content line as iCalendar bytes: fold at
/// [`MAX_LINE_OCTETS`] octets with a `{eol} ` continuation and terminate
/// with `eol`, mirroring calcard's writer.
fn render(content: &str, eol: &str) -> String {
    let mut out = String::with_capacity(content.len() + eol.len());
    let mut line_len = 0;

    for ch in content.chars() {
        let ch_len = ch.len_utf8();
        if line_len + ch_len > MAX_LINE_OCTETS {
            out.push_str(eol);
            out.push(' ');
            // The continuation space already fills one octet.
            line_len = 1;
        }
        out.push(ch);
        line_len += ch_len;
    }

    out.push_str(eol);
    out
}

/// The end of line of `raw` (its trailing terminator), defaulting to
/// CRLF when there is none.
fn eol_of(raw: &str) -> &str {
    if raw.ends_with("\r\n") {
        "\r\n"
    } else if raw.ends_with('\n') {
        "\n"
    } else {
        "\r\n"
    }
}

/// The property name of a content line: the characters up to the first
/// `;` or `:`, uppercased for matching. `None` for blank or nameless
/// lines.
fn property_name(content: &str) -> Option<String> {
    let end = content.find([';', ':'])?;
    let name = &content[..end];

    if name.is_empty() {
        return None;
    }

    Some(name.to_uppercase())
}

/// The component type of a `BEGIN:<type>` line, uppercased.
fn begin_name(content: &str) -> Option<String> {
    component_name(content, "BEGIN")
}

/// The component type of an `END:<type>` line, uppercased.
fn end_name(content: &str) -> Option<String> {
    component_name(content, "END")
}

/// The type that a `BEGIN`/`END` marker line names, when `content` is
/// such a marker (`marker` is `"BEGIN"` or `"END"`).
fn component_name(content: &str, marker: &str) -> Option<String> {
    if property_name(content)? != marker {
        return None;
    }

    let value = content.split_once(':')?.1.trim();
    Some(value.to_uppercase())
}

#[cfg(test)]
mod tests {
    use super::{Calendar, parse};

    const SAMPLE: &str = "BEGIN:VCALENDAR\r\n\
        VERSION:2.0\r\n\
        BEGIN:VEVENT\r\n\
        UID:abc@example\r\n\
        SUMMARY:Lunch\r\n\
        DTSTART;TZID=Europe/Paris:20260613T140000\r\n\
        X-FOO:bar\r\n\
        BEGIN:VALARM\r\n\
        ACTION:DISPLAY\r\n\
        TRIGGER:-PT15M\r\n\
        END:VALARM\r\n\
        END:VEVENT\r\n\
        END:VCALENDAR\r\n";

    fn applied(src: &str, name: &str, lines: &[&str]) -> String {
        let owned: Vec<String> = lines.iter().map(|line| line.to_string()).collect();
        let mut cal = parse(src);
        cal.component_mut("VEVENT").unwrap().set_all(name, &owned);
        cal.to_string()
    }

    #[test]
    fn round_trips_verbatim() {
        // CRLF, LF, a folded line, a bare event, two events and blanks.
        let folded = "BEGIN:VEVENT\r\nSUMMARY:a very long summary that has\r\n  been folded\r\nEND:VEVENT\r\n";
        let lf = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nSUMMARY:x\nEND:VEVENT\nEND:VCALENDAR\n";
        let bare = "BEGIN:VEVENT\r\nSUMMARY:x\r\nEND:VEVENT\r\n";
        let two = "BEGIN:VEVENT\r\nSUMMARY:a\r\nEND:VEVENT\r\nBEGIN:VEVENT\r\nSUMMARY:b\r\nEND:VEVENT\r\n";
        let blanks = "BEGIN:VEVENT\r\n\r\nSUMMARY:x\r\n\r\nEND:VEVENT\r\n";

        for src in [SAMPLE, folded, lf, bare, two, blanks] {
            assert_eq!(parse(src).to_string(), src);
        }
    }

    #[test]
    fn set_all_same_value_is_byte_identical() {
        assert_eq!(applied(SAMPLE, "SUMMARY", &["SUMMARY:Lunch"]), SAMPLE);
    }

    #[test]
    fn set_all_changes_only_one_line() {
        let out = applied(SAMPLE, "SUMMARY", &["SUMMARY:Team lunch"]);
        assert_eq!(out, SAMPLE.replace("SUMMARY:Lunch", "SUMMARY:Team lunch"));
    }

    #[test]
    fn set_all_long_value_folds() {
        let long = format!("DESCRIPTION:{}", "x".repeat(100));
        let out = applied(SAMPLE, "DESCRIPTION", &[&long]);

        // Folded into physical lines no wider than 75 octets, and the
        // rest of the calendar is left untouched.
        assert!(out.contains("\r\n "));
        for line in out.split("\r\n") {
            assert!(line.len() <= 75, "line too wide: {line:?}");
        }
        assert!(out.contains("SUMMARY:Lunch"));
    }

    #[test]
    fn set_all_empty_removes() {
        let out = applied(SAMPLE, "SUMMARY", &[]);
        assert!(!out.contains("SUMMARY:Lunch"));
        assert_eq!(out, SAMPLE.replace("SUMMARY:Lunch\r\n", ""));
    }

    #[test]
    fn set_all_inserts_before_subcomponents() {
        let out = applied(SAMPLE, "LOCATION", &["LOCATION:Room 1"]);
        let location = out.find("LOCATION:Room 1").unwrap();
        let alarm = out.find("BEGIN:VALARM").unwrap();
        assert!(location < alarm);
        // The new line carries the document end of line.
        assert!(out.contains("LOCATION:Room 1\r\n"));
    }

    #[test]
    fn set_all_resizes_a_group() {
        let one = applied(SAMPLE, "ATTENDEE", &["ATTENDEE:mailto:a@x"]);
        let three = applied(
            &one,
            "ATTENDEE",
            &[
                "ATTENDEE:mailto:a@x",
                "ATTENDEE:mailto:b@x",
                "ATTENDEE:mailto:c@x",
            ],
        );
        assert_eq!(three.matches("ATTENDEE:").count(), 3);

        let back = applied(&three, "ATTENDEE", &["ATTENDEE:mailto:a@x"]);
        assert_eq!(back.matches("ATTENDEE:").count(), 1);
    }

    #[test]
    fn mutation_leaves_siblings_and_alarms_untouched() {
        let two = "BEGIN:VCALENDAR\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:first\r\n\
            BEGIN:VALARM\r\n\
            TRIGGER:-PT15M\r\n\
            END:VALARM\r\n\
            END:VEVENT\r\n\
            BEGIN:VEVENT\r\n\
            SUMMARY:second\r\n\
            END:VEVENT\r\n\
            END:VCALENDAR\r\n";

        let out = applied(two, "SUMMARY", &["SUMMARY:edited"]);

        // Only the first event's summary changes.
        assert!(out.contains("SUMMARY:edited"));
        assert!(out.contains("SUMMARY:second"));
        // The nested alarm is byte-identical.
        assert!(out.contains("BEGIN:VALARM\r\nTRIGGER:-PT15M\r\nEND:VALARM"));
    }

    #[test]
    fn get_all_reads_direct_properties() {
        let mut cal: Calendar = parse(SAMPLE);
        let event = cal.component_mut("VEVENT").unwrap();
        assert_eq!(event.get_all("SUMMARY"), vec!["SUMMARY:Lunch"]);
        // The nested alarm's TRIGGER is not a direct property.
        assert!(event.get_all("TRIGGER").is_empty());
    }
}
