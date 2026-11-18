//! # Errors
//!
//! The crate-wide error and result types.

use core::{error, fmt, result};

use alloc::string::String;

/// The global `Error` enum of the library.
#[derive(Debug)]
pub enum TcalError {
    /// The input does not read as an iCalendar.
    ParseICalendar(String),
    /// One of a merge's three calendars does not read.
    ///
    /// It is named by the side it was given as, since the three are otherwise
    /// indistinguishable to the reader.
    ReadCalendar {
        /// The side the unreadable calendar was given as.
        side: &'static str,
        /// What ical-rs made of it.
        message: String,
    },
    /// A merged document still holds a collision.
    ///
    /// It is written as one key per side, which TOML refuses as a duplicate
    /// key, so an undecided document cannot be applied.
    Undecided(String),
    /// A date field holds a string that reads as no date.
    ///
    /// Written verbatim, it would make a property no iCalendar reader
    /// accepts. A value the calendar itself wrote is not refused: it is kept
    /// as it was, so only one typed in the document lands here.
    UnreadableDate {
        /// The TOML key of the date field.
        key: &'static str,
        /// The string it holds.
        value: String,
    },
    /// The edited TOML buffer is not valid TOML.
    ParseToml(toml_edit::TomlError),
    /// A requested component type key names no modelled component type.
    UnknownComponent(String),
    /// The document names an embedded attachment by a digest the calendar
    /// does not hold.
    UnknownEmbedded(String),
    /// A file the document names cannot be read.
    ReadFile {
        /// The path as the document wrote it.
        path: String,
        /// Why it could not be read.
        message: String,
    },
}

impl fmt::Display for TcalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseICalendar(message) => {
                write!(f, "Cannot parse iCalendar: {message}")
            }
            Self::ReadCalendar { side, message } => {
                write!(f, "Cannot read the {side} calendar: {message}")
            }
            Self::Undecided(key) => {
                write!(f, "Property {key} is left undecided: ")?;
                write!(f, "keep one of its lines and delete the others")
            }
            Self::UnreadableDate { key, value } => {
                write!(f, "Cannot read {value:?} as a date for {key}; ")?;
                write!(f, "expected a date-time like 2026-06-13T14:30:00")
            }
            Self::ParseToml(_) => {
                write!(f, "Cannot parse TOML buffer")
            }
            Self::UnknownComponent(key) => {
                write!(f, "Unknown component {key:?}; ")?;
                write!(f, "expected event, todo, journal, free-busy or timezone")
            }
            Self::UnknownEmbedded(sha256) => {
                write!(f, "Embedded attachment {sha256} is not in the calendar")
            }
            Self::ReadFile { path, message } => {
                write!(f, "Cannot read file {path}: {message}")
            }
        }
    }
}

impl TcalError {
    /// Whether the error refuses the edited document, which the reader can
    /// fix by editing it again: TOML that does not parse, a collision left
    /// undecided, a date that reads as none, a digest or a file path the
    /// document names.
    ///
    /// A calendar that does not read is not one: the reader cannot fix it
    /// from the document.
    pub fn is_document_refusal(&self) -> bool {
        match self {
            Self::ParseToml(_)
            | Self::Undecided(_)
            | Self::UnreadableDate { .. }
            | Self::UnknownEmbedded(_)
            | Self::ReadFile { .. } => true,
            Self::ParseICalendar(_) | Self::ReadCalendar { .. } | Self::UnknownComponent(_) => {
                false
            }
        }
    }
}

impl error::Error for TcalError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ParseToml(err) => Some(err),
            _ => None,
        }
    }
}

/// The global `Result` alias of the library.
pub type TcalResult<T> = result::Result<T, TcalError>;
