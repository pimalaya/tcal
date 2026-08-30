//! # Command-line interface
//!
//! The three verbs the `tcal` binary offers over the [`crate::template`]
//! projection, and how each resolves its source.
//!
//! - `template [SOURCE]`: print the TOML scaffold, blank or prefilled from an
//!   iCalendar. Always emits TOML.
//!
//! - `edit [SOURCE]`: project, open `$EDITOR`, fold the edits back onto the
//!   source, and emit the resulting iCalendar.
//!
//! - `merge BASE LOCAL REMOTE OUTPUT`: merge two divergent calendars against
//!   their base ([`crate::merge`]), edit the result, write it out once decided.
//!
//! `SOURCE` resolves deterministically: `-` reads stdin, an existing file is
//! read, otherwise the value is treated as literal iCalendar contents, and
//! omitting it starts from a blank template.
//!
//! The TOML is an editing affordance, never an interchange format, so the only
//! path back to an iCalendar is `edit`, where the original is still in hand.

use alloc::{borrow::ToOwned, format, string::String, vec::Vec};

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
    prompt,
};
use uuid::Uuid;

use crate::{
    error::{self, TcalError},
    ical,
    merge::Merge,
    template,
};

/// Root CLI parser.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(infer_subcommands = true)]
pub struct Cli {
    /// The verb to run.
    #[command(subcommand)]
    pub cmd: Command,
    /// Whether output and errors are rendered as JSON rather than as text.
    #[command(flatten)]
    pub json: JsonFlag,
    /// Where and how verbosely the logger writes.
    ///
    /// tcal emits no records of its own, so these only surface what the
    /// libraries under it write.
    #[command(flatten)]
    pub log: LogFlags,
}

/// Top-level subcommands.
///
/// A variant carries no documentation of its own: it delegates to the command
/// type it wraps, whose docs are what clap renders as that subcommand's help.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(visible_alias = "tpl")]
    Template(TemplateCommand),
    Edit(EditCommand),
    Merge(MergeCommand),
    #[command(alias = "completions")]
    Completion(CompletionCommand),
    #[command(alias = "manuals")]
    Manual(ManualCommand),
}

impl Command {
    /// Run the parsed subcommand.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        match self {
            Self::Template(cmd) => cmd.execute(printer),
            Self::Edit(cmd) => cmd.execute(printer),
            Self::Merge(cmd) => cmd.execute(printer),
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
        }
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

/// Print a TOML template, blank or prefilled from an iCalendar.
#[derive(Debug, Parser)]
pub struct TemplateCommand {
    /// The iCalendar to prefill the form from.
    #[command(flatten)]
    pub source: SourceArg,
    /// The component types the form shows.
    #[command(flatten)]
    pub components: ComponentFlags,
    /// Write to this file instead of stdout.
    #[arg(short, long, value_name = "PATH", value_parser = path_parser)]
    pub output: Option<PathBuf>,
}

impl TemplateCommand {
    /// Project the source and write the TOML out.
    pub fn execute(self, _printer: &mut impl Printer) -> Result<()> {
        let src = load(&self.source)?;
        let ical = ical::parse(&src)?;
        let toml = template::project_with(&ical, &self.components.selected())?;
        write_out(self.output.as_deref(), toml.as_bytes())
    }
}

/// Edit an iCalendar as TOML in `$EDITOR`, blank or prefilled from a source.
#[derive(Debug, Parser)]
pub struct EditCommand {
    /// The iCalendar to prefill the form from, and to fold the edits onto.
    #[command(flatten)]
    pub source: SourceArg,
    /// The component types the form shows, and the only ones it reconciles.
    #[command(flatten)]
    pub components: ComponentFlags,
    /// Write the resulting iCalendar here instead of stdout.
    ///
    /// Editing a file writes it back in place, so this is what redirects the
    /// result elsewhere.
    #[arg(short, long, value_name = "PATH", value_parser = path_parser)]
    pub output: Option<PathBuf>,
}

impl EditCommand {
    /// Project the source, edit it, then write the folded-back iCalendar out.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        let src = load(&self.source)?;
        let ical = ical::parse(&src)?;
        let types = self.components.selected();
        let toml = template::project_with(&ical, &types)?;

        let out = edit_until_applied(printer, &toml, |edited| {
            template::apply_with(&src, edited, &types)
        })?;

        let target = self.output.or_else(|| self.source.file_path());
        write_out(target.as_deref(), out.as_bytes())
    }
}

/// Merge two divergent calendars against their base and edit the result.
///
/// Collisions the merge could not settle are written as the same key once
/// per side, which TOML refuses: decide each one by keeping a single line.
/// The output is written only once the edited document parses.
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
    /// Where to write the resolved calendar.
    #[arg(value_name = "OUTPUT", value_parser = path_parser)]
    pub output: PathBuf,
}

impl MergeCommand {
    /// Merge the three calendars, edit the outcome, then write it out.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        let base = read_calendar(&self.base)?;
        let local = read_calendar(&self.local)?;
        let remote = read_calendar(&self.remote)?;

        let merged = Merge {
            base: &base,
            local: &local,
            remote: &remote,
        }
        .project()?;

        let out = edit_until_applied(printer, &merged.toml, |edited| merged.apply(edited))?;

        // NOTE: the output is written only here, so a document left undecided
        // or an editor left unanswered leaves it as it was.
        write_out(Some(&self.output), out.as_bytes())
    }
}

/// Open the editor on a projected buffer and fold the result back.
///
/// A buffer that comes back unusable is re-opened as the user left it, so the
/// edits are never lost. JSON output is non-interactive, so the error
/// propagates there instead.
fn edit_until_applied(
    printer: &impl Printer,
    toml: &str,
    apply: impl Fn(&str) -> error::Result<String>,
) -> Result<String> {
    let mut builder = edit::Builder::new();
    builder.suffix(".toml");

    let mut edited = edit::edit_with_builder(toml, &builder).context("Cannot spawn editor")?;

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

/// Read one of a merge's calendars from a path.
fn read_calendar(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("Cannot read iCalendar {path:?}"))
}

/// Positional iCalendar source shared by both verbs.
#[derive(Debug, Parser)]
pub struct SourceArg {
    /// A path to an iCalendar file, raw iCalendar contents, or `-` for stdin.
    ///
    /// Omit it to start from a blank template.
    #[arg(value_name = "SOURCE")]
    pub source: Option<String>,
}

impl SourceArg {
    /// Resolve the source into iCalendar text, `None` for a blank template.
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

    /// The source as an existing file path, when it resolves to one.
    ///
    /// This is the in-place write default of `edit`.
    fn file_path(&self) -> Option<PathBuf> {
        let source = self.source.as_ref()?;

        if source == "-" {
            return None;
        }

        let path = path_parser(source).ok()?;
        path.is_file().then_some(path)
    }
}

/// Load the raw source iCalendar text, seeding a fresh one when blank.
///
/// Returning the original text rather than a parsed model is what lets
/// [`template::apply`] preserve every untouched byte.
fn load(source: &SourceArg) -> Result<String> {
    match source.resolve()? {
        Some(text) => Ok(text),
        None => {
            // NOTE: a new event is seeded with a fresh UID and DTSTAMP so the
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
