//! # Errors
//!
//! The crate-wide error and result types.

use core::result;

use alloc::string::String;
use thiserror::Error;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum TcalError {
    /// The input does not read as an iCalendar.
    #[error("Cannot parse iCalendar: {0}")]
    ParseICalendar(String),
    /// One of a merge's three calendars does not read, named by the side it
    /// was given as.
    #[error("Cannot read the {side} calendar: {message}")]
    ReadCalendar {
        /// The side the unreadable calendar was given as.
        side: &'static str,
        /// What ical-rs made of it.
        message: String,
    },
    /// A merged document still holds a collision, written as one key per
    /// side, which TOML refuses as a duplicate key.
    #[error("Property {0} is left undecided: keep one of its lines and delete the others")]
    Undecided(String),
    /// The edited TOML buffer is not valid TOML.
    #[error("Cannot parse TOML buffer")]
    ParseToml(#[source] toml_edit::TomlError),
    /// A requested component type key names no modeled component type.
    #[error("Unknown component {0:?}; expected event, todo, journal, free-busy or timezone")]
    UnknownComponent(String),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, TcalError>;
