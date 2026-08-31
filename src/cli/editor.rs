//! # Editor round trip
//!
//! Opening `$EDITOR` on a TOML document and folding what comes back onto the
//! calendar it was projected from.
//!
//! A document that does not fold back re-opens as the reader left it, so an
//! edit is never lost, whether it is broken TOML or a collision still to
//! decide.

use alloc::{format, string::String};

use anyhow::{Context, Result};
use log::info;
use pimalaya_cli::{printer::Printer, prompt};

use crate::error::{TcalError, TcalResult};

/// The `$EDITOR` round trip over one TOML document.
pub struct Editor<'a> {
    /// The document the editor opens on.
    pub document: &'a str,
}

impl Editor<'_> {
    /// Edit the document, then fold it back with `apply`.
    ///
    /// A failed fold offers to re-open the editor on the buffer that failed,
    /// looping until it applies or the reader declines. JSON output is
    /// non-interactive and propagates the error instead.
    pub fn apply(
        &self,
        printer: &impl Printer,
        apply: impl Fn(&str) -> TcalResult<String>,
    ) -> Result<String> {
        let mut builder = edit::Builder::new();
        builder.suffix(".toml");

        info!("opening editor on the projected document");
        let mut edited =
            edit::edit_with_builder(self.document, &builder).context("Cannot spawn editor")?;

        loop {
            let err = match apply(&edited) {
                Ok(out) => return Ok(out),
                Err(err) => err,
            };

            let message = match &err {
                TcalError::ParseToml(err) if !printer.is_json() => {
                    format!("Cannot parse TOML buffer:\n\n{err}\nRe-edit to fix it?")
                }
                TcalError::Undecided(key) if !printer.is_json() => format!(
                    "Property {key} is left undecided.\n\nKeep one of its lines and delete the others. Re-edit to decide it?"
                ),
                _ => return Err(err.into()),
            };

            if !prompt::bool(message, true)? {
                return Err(err.into());
            }

            edited = edit::edit_with_builder(&edited, &builder).context("Cannot spawn editor")?;
        }
    }
}
