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
    /// The edited TOML buffer is not valid TOML.
    ParseToml(toml_edit::TomlError),
    /// A requested component type key names no modelled component type.
    UnknownComponent(String),
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
            Self::ParseToml(_) => {
                write!(f, "Cannot parse TOML buffer")
            }
            Self::UnknownComponent(key) => {
                write!(f, "Unknown component {key:?}; ")?;
                write!(f, "expected event, todo, journal, free-busy or timezone")
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
