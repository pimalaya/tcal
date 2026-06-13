//! Thin adapter over [`calcard`]: parse raw iCalendar text into an
//! [`ICalendar`].
//!
//! calcard owns the model and the writer; tcal never hand-builds
//! entries. Parse here, project to TOML in [`crate::template`], then
//! let calcard serialize the result back.

use calcard::{Entry, icalendar::ICalendar};

use crate::error::{Result, TcalError};

/// Parse raw iCalendar text into a calcard [`ICalendar`].
///
/// A bare `VEVENT` (or any standalone component) is accepted as well as
/// a full `VCALENDAR`; only a genuine failure or a vCard payload is
/// rejected.
pub fn parse(input: &str) -> Result<ICalendar> {
    match ICalendar::parse(input) {
        Ok(ical) => Ok(ical),
        Err(Entry::VCard(_)) => Err(TcalError::NotAnICalendar),
        Err(Entry::InvalidLine(line)) => Err(TcalError::ParseICalendar(line)),
        Err(other) => Err(TcalError::ParseICalendar(format!("{other:?}"))),
    }
}
