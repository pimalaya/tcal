//! Thin adapter over [`calcard`]: parse raw vCard text into a
//! [`VCard`].
//!
//! calcard owns the model and the writer; tcard never hand-builds
//! entries. Parse here, project to TOML in [`crate::template`], then
//! let calcard serialize the result back.

use calcard::{Entry, vcard::VCard};

use crate::error::{Result, TcardError};

/// Parse raw vCard text into a calcard [`VCard`].
///
/// A vCard that parses with trailing issues is still returned (via
/// calcard's `Err(Entry::VCard(_))` recovery path); only a genuine
/// failure or an iCalendar payload is rejected.
pub fn parse(input: &str) -> Result<VCard> {
    match VCard::parse(input) {
        Ok(vcard) => Ok(vcard),
        Err(Entry::VCard(vcard)) => Ok(vcard),
        Err(Entry::ICalendar(_)) => Err(TcardError::NotAVcard),
        Err(Entry::InvalidLine(line)) => Err(TcardError::ParseVcard(line)),
        Err(other) => Err(TcardError::ParseVcard(format!("{other:?}"))),
    }
}
