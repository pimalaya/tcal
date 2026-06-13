//! The crate-wide error and result types.

use std::result;

use thiserror::Error;

/// The global `Error` enum of the library.
#[derive(Debug, Error)]
pub enum TcalError {
    /// calcard parsed the input as a vCard instead of an iCalendar.
    #[error("Contents parsed as a vCard, not an iCalendar")]
    NotAnICalendar,
    /// calcard could not parse the input as an iCalendar.
    #[error("Cannot parse iCalendar: {0}")]
    ParseICalendar(String),
    /// The edited TOML buffer is not valid TOML.
    #[error("Cannot parse TOML buffer")]
    ParseToml(#[source] toml_edit::TomlError),
    /// The iCalendar carries no VEVENT to fold the edits back onto.
    #[error("No VEVENT component found in the iCalendar")]
    NoEvent,
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, TcalError>;
