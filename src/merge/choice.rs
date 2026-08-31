//! # Choices
//!
//! One contested key: where it sits in the document, and how each side spells
//! it.
//!
//! A collision the merge already settled, and one on a part the projection
//! does not show, make no choice: they have no line to contest and are said in
//! the document's header instead.

use alloc::{format, string::String, vec::Vec};

/// One contested key.
///
/// It carries where it sits in the document, and how each side spells it.
pub struct Choice {
    /// The block the contested lines sit in, one step per array of tables.
    ///
    /// Each step is the TOML key and the index of the block among its
    /// siblings.
    pub at: Vec<(&'static str, usize)>,
    /// The field key whose lines are contested.
    ///
    /// It is every key of the block for an attendee, which the projection
    /// writes as a table of its own.
    pub key: Option<&'static str>,
    /// The comment this becomes where the document holds no line to contest.
    ///
    /// A collision with nowhere to go is still said.
    pub kept: String,
    /// The ancestor's lines, commented above the choice.
    pub base: Vec<String>,
    /// The local side's lines.
    pub local: Vec<String>,
    /// The remote side's lines.
    pub remote: Vec<String>,
}

impl Choice {
    /// Whether a projected line writes this choice's contested key.
    pub fn contests(&self, line: &str) -> bool {
        let Some((key, _)) = line.split_once('=') else {
            return false;
        };

        let key = key.trim();

        match self.key {
            None => !key.is_empty(),
            Some(field) => {
                key == field
                    || key
                        .strip_prefix(field)
                        .is_some_and(|rest| rest.starts_with('.') || rest == "-tz")
            }
        }
    }

    /// Every key the two sides write between them.
    ///
    /// They come in the order the local side writes them, a key only the
    /// remote side has last.
    fn keys(&self) -> Vec<&str> {
        let mut keys: Vec<&str> = Vec::new();

        for line in self.local.iter().chain(&self.remote) {
            let key = key_of(line);

            if !keys.contains(&key) {
                keys.push(key);
            }
        }

        keys
    }

    /// The keys the two sides spell differently.
    ///
    /// Those are the ones only a reader can settle.
    pub fn contested(&self) -> Vec<&str> {
        self.keys()
            .into_iter()
            .filter(|key| line_for(&self.local, key) != line_for(&self.remote, key))
            .collect()
    }

    /// Write the contest over the lines it holds.
    ///
    /// A key the sides differ on becomes the ancestor as a comment then one
    /// live line per side, each naming the side it came from. A key they
    /// agree on is written once, either copy being the other.
    pub fn render(&self, out: &mut String) {
        let contested = self.contested();

        out.push_str(if contested.len() < 2 {
            "# conflict, keep one line\n"
        } else {
            "# conflict, keep one side\n"
        });

        for key in self.keys() {
            if !contested.contains(&key) {
                if let Some(line) = line_for(&self.local, key) {
                    out.push_str(&format!("{line}\n"));
                }

                continue;
            }

            if let Some(line) = line_for(&self.base, key) {
                out.push_str(&format!("# {line} # base\n"));
            }

            if let Some(line) = line_for(&self.local, key) {
                out.push_str(&format!("{line} # local\n"));
            }

            if let Some(line) = line_for(&self.remote, key) {
                out.push_str(&format!("{line} # remote\n"));
            }
        }
    }
}

/// The key a projected line writes, which is its text up to the `=`.
pub fn key_of(line: &str) -> &str {
    line.split_once('=').map_or(line, |(key, _)| key).trim()
}

/// The line one side writes for a key, absent where that side writes none.
fn line_for<'l>(lines: &'l [String], key: &str) -> Option<&'l str> {
    lines
        .iter()
        .find(|line| key_of(line) == key)
        .map(String::as_str)
}
