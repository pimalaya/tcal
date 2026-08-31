//! # Merge command
//!
//! Merging two divergent calendars against their base, then deciding the rest
//! in `$EDITOR`. It takes three paths rather than a source, a merge needing
//! three calendars at once.

use alloc::{format, string::String};

use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use clap::Parser;
use log::info;
use pimalaya_cli::{clap::parsers::path_parser, printer::Printer};

use crate::{
    cli::{
        args::{EditorArg, Output},
        editor::Editor,
    },
    merge::TcalMerge,
};

/// Merge two divergent calendars against their base and edit the result.
///
/// Collisions the merge could not settle are written as the same key once per
/// side, which TOML refuses: decide each one by keeping a single line. The
/// output is written only once the edited document parses.
#[derive(Debug, Parser)]
pub struct MergeCommand {
    /// The common ancestor both sides were derived from.
    #[arg(value_name = "BASE", value_parser = path_parser)]
    pub base: PathBuf,
    /// The edited side, whose changes are replayed onto the remote one.
    #[arg(value_name = "LOCAL", value_parser = path_parser)]
    pub local: PathBuf,
    /// The other side.
    #[arg(value_name = "REMOTE", value_parser = path_parser)]
    pub remote: PathBuf,
    /// Write the merged calendar here, once the document is decided.
    #[arg(short, long, value_name = "PATH", value_parser = path_parser)]
    pub output: PathBuf,
    /// The editor the document is decided in.
    #[command(flatten)]
    pub editor: EditorArg,
}

impl MergeCommand {
    /// Merge the three calendars, edit the outcome, then write it out.
    ///
    /// The output is written once, at the end, so a document left undecided or
    /// an editor left unanswered leaves it as it was.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        let base = read(&self.base)?;
        let local = read(&self.local)?;
        let remote = read(&self.remote)?;

        let merged = TcalMerge {
            base: &base,
            local: &local,
            remote: &remote,
        }
        .project()?;

        let editor = Editor {
            document: &merged.toml,
            command: self.editor.editor.as_deref(),
        };
        let out = editor.apply(printer, |edited| merged.apply(edited))?;

        Output(Some(&self.output)).write(out.as_bytes())
    }
}

/// Read one of the three calendars a merge takes.
fn read(path: &PathBuf) -> Result<String> {
    info!("reading iCalendar from {path:?}");
    fs::read_to_string(path).with_context(|| format!("Cannot read iCalendar {path:?}"))
}
