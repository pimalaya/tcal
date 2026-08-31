//! # Shared arguments
//!
//! The source a verb reads its calendar from, the component types it shows,
//! and the file it writes to.

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

use std::{
    fs,
    io::{Read, Write, stdin, stdout},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::Parser;
use log::{debug, info};
use pimalaya_cli::clap::parsers::path_parser;
use uuid::Uuid;

/// Positional iCalendar source shared by the template and edit verbs.
#[derive(Debug, Parser)]
pub struct SourceArg {
    /// A path to an iCalendar file, raw iCalendar contents, or `-` for stdin.
    ///
    /// Omit it to start from a blank template.
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,
}

impl SourceArg {
    /// The iCalendar text a verb reads, seeding a new one where there is none.
    ///
    /// A new event is given a fresh `UID` and `DTSTAMP`, so what comes out is
    /// a valid `VEVENT` from the start.
    ///
    /// The raw text is what comes back rather than a parsed model, which is
    /// what lets a fold-back preserve every untouched byte.
    pub fn load(&self) -> Result<String> {
        if let Some(text) = self.resolve()? {
            return Ok(text);
        }

        info!("seeding a new event with a fresh UID and DTSTAMP");
        let stamp = Utc::now().format("%Y%m%dT%H%M%SZ");

        Ok(format!(
            "BEGIN:VCALENDAR\r\n\
             VERSION:2.0\r\n\
             PRODID:-//Pimalaya//tcal//EN\r\n\
             BEGIN:VEVENT\r\n\
             UID:{}\r\n\
             DTSTAMP:{stamp}\r\n\
             END:VEVENT\r\n\
             END:VCALENDAR\r\n",
            Uuid::new_v4()
        ))
    }

    /// The source as an existing file path, when it resolves to one.
    ///
    /// This is the in-place write default of `edit`.
    pub fn file_path(&self) -> Option<PathBuf> {
        let source = self.source.as_ref()?;

        if source == "-" {
            return None;
        }

        let path = path_parser(source).ok()?;
        path.is_file().then_some(path)
    }

    /// Resolve the source into iCalendar text, `None` for a blank template.
    fn resolve(&self) -> Result<Option<String>> {
        let Some(source) = &self.source else {
            return Ok(None);
        };

        if source == "-" {
            info!("reading iCalendar from stdin");
            let mut buffer = String::new();
            stdin()
                .read_to_string(&mut buffer)
                .context("Cannot read iCalendar from stdin")?;
            return Ok(Some(buffer));
        }

        if let Some(path) = self.file_path() {
            info!("reading iCalendar from {path:?}");
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Cannot read iCalendar {path:?}"))?;
            return Ok(Some(contents));
        }

        let trimmed = source.trim_start();

        if trimmed.starts_with("BEGIN:VCALENDAR") || trimmed.starts_with("BEGIN:VEVENT") {
            debug!("treating source as literal iCalendar contents");
            return Ok(Some(source.clone()));
        }

        bail!("Source {source:?} is neither a readable file nor iCalendar contents")
    }
}

/// Selection of component types to show, by flag.
///
/// Cumulative: none shows the whole calendar, one flattens that type as the
/// document root, and two or more keep the VCALENDAR root and show only those.
/// A type left unselected is also left untouched on save.
#[derive(Debug, Parser)]
pub struct ComponentFlags {
    /// Show events (VEVENT).
    #[arg(long)]
    pub event: bool,
    /// Show to-dos (VTODO).
    #[arg(long)]
    pub todo: bool,
    /// Show journals (VJOURNAL).
    #[arg(long)]
    pub journal: bool,
    /// Show free/busy reports (VFREEBUSY).
    #[arg(long)]
    pub free_busy: bool,
    /// Show time zones (VTIMEZONE).
    #[arg(long)]
    pub timezone: bool,
}

impl ComponentFlags {
    /// The selected component type keys, in a stable order.
    ///
    /// Empty means no filter, so every type is shown.
    pub fn selected(&self) -> Vec<String> {
        [
            (self.event, "event"),
            (self.todo, "todo"),
            (self.journal, "journal"),
            (self.free_busy, "free-busy"),
            (self.timezone, "timezone"),
        ]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, key)| key.to_owned())
        .collect()
    }
}

/// Where a verb writes its result, stdout when it has no path.
pub struct Output<'a>(pub Option<&'a Path>);

impl Output<'_> {
    /// Write the bytes out, creating or truncating a file target.
    pub fn write(&self, bytes: &[u8]) -> Result<()> {
        match self.0 {
            Some(path) => {
                info!("writing {} bytes to {path:?}", bytes.len());
                fs::write(path, bytes).with_context(|| format!("Cannot write to {path:?}"))
            }
            None => {
                info!("writing {} bytes to stdout", bytes.len());
                stdout().write_all(bytes).context("Cannot write to stdout")
            }
        }
    }
}

/// The editor a round trip opens, shared by the edit and merge verbs.
#[derive(Debug, Parser)]
pub struct EditorArg {
    /// Command the document is edited in, winning over `$VISUAL` and
    /// `$EDITOR`.
    ///
    /// Spawned on the path of a temporary TOML file it edits in place, so it
    /// must block until the edit is done: use `code --wait`, not `code`.
    #[arg(short, long, value_name = "COMMAND")]
    pub editor: Option<String>,
}
