//! # Decorated document
//!
//! The projected TOML document, with what the merge could not settle written
//! into it.
//!
//! An undecided collision replaces the lines it contests, so the same key
//! appears twice and the document stops parsing. Everything else the merge did
//! on its own is said in the header comment.

use alloc::{borrow::ToOwned, format, string::String, vec, vec::Vec};

use crate::merge::choice::Choice;

/// The column a header comment wraps at, its `# ` prefix included.
const WRAP: usize = 66;

/// A projection, and what a merge has to write into it.
pub struct Document<'a> {
    /// The projection of the merged calendar.
    pub toml: &'a str,
    /// What the merge settled on its own, said in the header.
    pub notes: Vec<String>,
    /// What only a reader can settle, written over the lines it contests.
    pub choices: Vec<Choice>,
}

impl Document<'_> {
    /// Write the choices over the lines they contest, then the notes above
    /// them.
    ///
    /// The body is written first so the header can announce the contests the
    /// document holds rather than the ones the merge reported: a choice the
    /// body found no line for falls back to the note it would have been.
    pub fn decorate(&self) -> String {
        let mut header = String::new();
        let mut body = String::new();
        let mut here: Vec<(&str, usize)> = Vec::new();
        let mut counts: Vec<(&str, usize)> = Vec::new();
        let mut written = vec![false; self.choices.len()];

        for line in self.toml.lines() {
            // NOTE: the projection's own header is every comment line the
            // document opens with, and the notes go under it.
            if body.is_empty() && line.starts_with('#') {
                header.push_str(line);
                header.push('\n');
                continue;
            }

            if let Some(block) = block_header(line) {
                open(&mut here, &mut counts, block);
                body.push_str(line);
                body.push('\n');
                continue;
            }

            let contested = self
                .choices
                .iter()
                .position(|choice| addresses(&choice.at, &here) && choice.contests(line));

            match contested {
                Some(at) if !written[at] => {
                    self.choices[at].render(&mut body);
                    written[at] = true;
                }
                // NOTE: a choice writes every side's lines at once, so the
                // lines it replaces are dropped rather than written again.
                Some(_) => {}
                None => {
                    body.push_str(line);
                    body.push('\n');
                }
            }
        }

        let mut said: Vec<&str> = self.notes.iter().map(String::as_str).collect();

        said.extend(
            self.choices
                .iter()
                .zip(&written)
                .filter(|(_, written)| !**written)
                .map(|(choice, _)| choice.kept.as_str()),
        );

        let mut out = header;

        preamble(&mut out, &said, written.iter().filter(|w| **w).count());
        out.push_str(&body);

        out
    }
}

/// Write what the reader is asked, then what the merge settled without asking.
///
/// Both go as comments under the projection's own header.
fn preamble(out: &mut String, notes: &[&str], contests: usize) {
    if contests == 0 && notes.is_empty() {
        return;
    }

    out.push_str("#\n");

    if contests > 0 {
        let plural = if contests == 1 { "" } else { "s" };

        comment(
            out,
            &format!(
                "{contests} conflict{plural} below {} yours to decide, written as the same key once per side. Keep one line of each and delete the others, or replace them with a value of your own: TOML forbids duplicate keys, so this document cannot be applied until every one is decided.",
                if contests == 1 { "is" } else { "are" }
            ),
        );
    }

    if !notes.is_empty() {
        if contests > 0 {
            out.push_str("#\n");
        }

        comment(out, "The merge settled these on its own:");
        out.push_str("#\n");

        for note in notes {
            comment(out, &format!("- {note}"));
        }
    }

    out.push_str("#\n");
}

/// Write one comment paragraph, wrapped at [`WRAP`].
///
/// A continuation line of a bullet is indented under its text.
fn comment(out: &mut String, text: &str) {
    let indent = if text.starts_with("- ") { "  " } else { "" };
    let mut line = String::new();

    for word in text.split_whitespace() {
        if !line.is_empty() && line.len() + word.len() + 1 > WRAP {
            out.push_str(&format!("# {line}\n"));
            line = indent.to_owned();
        }

        if !line.is_empty() && !line.ends_with(' ') {
            line.push(' ');
        }

        line.push_str(word);
    }

    if !line.trim().is_empty() {
        out.push_str(&format!("# {line}\n"));
    }
}

/// The dotted key of an array of tables header, for a line that is one.
fn block_header(line: &str) -> Option<&str> {
    line.strip_prefix("[[")?.strip_suffix("]]")
}

/// Open a block at the depth its header names.
///
/// The last step of that header becomes the current block there, and every
/// counter nested under it starts again, since those blocks belong to the one
/// just opened.
fn open<'t>(here: &mut Vec<(&'t str, usize)>, counts: &mut Vec<(&'t str, usize)>, header: &'t str) {
    let depth = header.split('.').count();

    let index = match counts.iter_mut().find(|(held, _)| *held == header) {
        Some((_, count)) => {
            *count += 1;
            *count
        }
        None => {
            counts.push((header, 0));
            0
        }
    };

    counts.retain(|(held, _)| {
        !held
            .strip_prefix(header)
            .is_some_and(|rest| rest.starts_with('.'))
    });

    here.truncate(depth - 1);
    here.push((header.rsplit('.').next().unwrap_or(header), index));
}

/// Whether a choice's address is the block currently open.
fn addresses(at: &[(&str, usize)], here: &[(&str, usize)]) -> bool {
    at.len() == here.len() && at.iter().zip(here).all(|(at, here)| at == here)
}
