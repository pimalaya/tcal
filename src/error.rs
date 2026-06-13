//! The crate-wide error and result types.

use core::result;

use alloc::string::String;
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
    /// A requested component type key names no modeled component type.
    #[error("Unknown component {0:?}; expected event, todo, journal, free-busy or timezone")]
    UnknownComponent(String),
}

/// The global `Result` alias of the library.
pub type Result<T> = result::Result<T, TcalError>;
