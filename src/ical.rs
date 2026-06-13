//! Thin calcard adapter: parse raw iCalendar text into an [`ICalendar`].

use alloc::format;

use calcard::{Entry, icalendar::ICalendar};

use crate::error::{Result, TcalError};

/// Parse raw iCalendar text into a calcard [`ICalendar`]. A bare component
/// is accepted as well as a full `VCALENDAR`; a vCard payload is rejected.
pub fn parse(input: &str) -> Result<ICalendar> {
    match ICalendar::parse(input) {
        Ok(ical) => Ok(ical),
        Err(Entry::VCard(_)) => Err(TcalError::NotAnICalendar),
        Err(Entry::InvalidLine(line)) => Err(TcalError::ParseICalendar(line)),
        Err(other) => Err(TcalError::ParseICalendar(format!("{other:?}"))),
    }
}
