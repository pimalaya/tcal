//! The `tcal` binary: two verbs over the [`crate::template`] projection.
//!
//! - `template [SOURCE]`: print the TOML scaffold, blank or prefilled
//!   from an iCalendar. Always emits TOML.
//! - `edit [SOURCE]`: project, open `$EDITOR`, apply the edits back onto
//!   the source, and emit the resulting iCalendar. Always emits an
//!   iCalendar.
//!
//! `SOURCE` resolves deterministically: `-` reads stdin, an existing
//! file is read, otherwise the value is treated as literal iCalendar
//! contents, and omitting it starts from a blank template. The TOML is
//! an editing affordance; the only path back to an iCalendar is `edit`,
//! where the original is still in hand.

use std::{
    fs,
    io::{Read, Write, stdin, stdout},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{JsonFlag, LogFlags},
        commands::{CompletionCommand, ManualCommand},
        parsers::path_parser,
    },
    long_version,
    printer::Printer,
};
use uuid::Uuid;

use crate::{ical, template};

/// Root CLI parser.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(infer_subcommands = true)]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Command,

    #[command(flatten)]
    pub json: JsonFlag,
    #[command(flatten)]
    pub log: LogFlags,
}

/// Top-level subcommands.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(visible_alias = "tpl")]
    Template(TemplateCommand),
    Edit(EditCommand),

    Completions(CompletionCommand),
    Manuals(ManualCommand),
}

impl Command {
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        match self {
            Self::Template(cmd) => cmd.execute(printer),
            Self::Edit(cmd) => cmd.execute(printer),
            Self::Completions(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manuals(cmd) => cmd.execute(printer, Cli::command()),
        }
    }
}

/// Print a TOML template, blank or prefilled from an iCalendar.
#[derive(Debug, Parser)]
pub struct TemplateCommand {
    #[command(flatten)]
    pub source: SourceArg,

    /// Write to this file instead of stdout.
    #[arg(short, long, value_name = "PATH", value_parser = path_parser)]
    pub output: Option<PathBuf>,
}

impl TemplateCommand {
    pub fn execute(self, _printer: &mut impl Printer) -> Result<()> {
        let src = load(&self.source)?;
        let ical = ical::parse(&src)?;
        let toml = template::project(&ical);

        write_out(self.output.as_deref(), toml.as_bytes())
    }
}

/// Edit an iCalendar as TOML in `$EDITOR`, blank or prefilled from a
/// source.
#[derive(Debug, Parser)]
pub struct EditCommand {
    #[command(flatten)]
    pub source: SourceArg,

    /// Write the resulting iCalendar here instead of stdout (or the
    /// source file, when editing one in place).
    #[arg(short, long, value_name = "PATH", value_parser = path_parser)]
    pub output: Option<PathBuf>,
}

impl EditCommand {
    pub fn execute(self, _printer: &mut impl Printer) -> Result<()> {
        let src = load(&self.source)?;
        let ical = ical::parse(&src)?;
        let scaffold = template::project(&ical);

        let edited = edit::edit_with_builder(&scaffold, edit::Builder::new().suffix(".toml"))
            .context("Cannot spawn editor")?;

        let out = template::apply(&src, &edited)?;

        let target = self.output.or_else(|| self.source.file_path());
        write_out(target.as_deref(), out.as_bytes())
    }
}

/// Positional iCalendar source shared by both verbs.
#[derive(Debug, Parser)]
pub struct SourceArg {
    /// A path to an iCalendar file, raw iCalendar contents, or `-` for
    /// stdin. Omit to start from a blank template.
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,
}

impl SourceArg {
    /// Resolve the source into iCalendar text, or `None` for a blank
    /// template.
    pub fn resolve(&self) -> Result<Option<String>> {
        let Some(source) = &self.source else {
            return Ok(None);
        };

        if source == "-" {
            let mut buffer = String::new();
            stdin()
                .read_to_string(&mut buffer)
                .context("Cannot read iCalendar from stdin")?;
            return Ok(Some(buffer));
        }

        if let Some(path) = self.file_path() {
            let contents = fs::read_to_string(&path)
                .with_context(|| format!("Cannot read iCalendar {path:?}"))?;
            return Ok(Some(contents));
        }

        let trimmed = source.trim_start();
        if trimmed.starts_with("BEGIN:VCALENDAR") || trimmed.starts_with("BEGIN:VEVENT") {
            return Ok(Some(source.clone()));
        }

        bail!("Source {source:?} is neither a readable file nor iCalendar contents")
    }

    /// The source as an existing file path, when it resolves to one;
    /// used for the in-place write default of `edit`.
    fn file_path(&self) -> Option<PathBuf> {
        let source = self.source.as_ref()?;

        if source == "-" {
            return None;
        }

        let path = path_parser(source).ok()?;
        path.is_file().then_some(path)
    }
}

/// Load the raw source iCalendar text, or seed a fresh one for a blank
/// template. Returning the original text (not a parsed model) lets
/// [`template::apply`] preserve every untouched byte.
fn load(source: &SourceArg) -> Result<String> {
    match source.resolve()? {
        Some(text) => Ok(text),
        None => {
            // A new event is seeded with a fresh UID and DTSTAMP so the
            // result is a valid VEVENT from the start.
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
    }
}

/// Write bytes to a file, or to stdout when no path is given.
fn write_out(path: Option<&Path>, bytes: &[u8]) -> Result<()> {
    match path {
        Some(path) => fs::write(path, bytes).with_context(|| format!("Cannot write to {path:?}")),
        None => stdout().write_all(bytes).context("Cannot write to stdout"),
    }
}
