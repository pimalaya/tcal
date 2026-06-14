//! A format-preserving iCalendar editor, the `toml_edit` analog for
//! iCalendar.
//!
//! calcard is a normalizing reader/writer: re-serializing churns line
//! folding, parameter casing and property order even where nothing changed.
//! This editor instead keeps every content line's original bytes and
//! re-renders only the lines a caller mutates ([`tree`]), so editing one
//! property yields a minimal diff. iCalendar is line-oriented with a single
//! wrinkle, line folding ([`parse`], [`render`]). It is calcard-independent
//! (std only); the core invariant is `Calendar::parse(s).to_string() == s`.

mod parse;
mod render;
pub mod tree;
