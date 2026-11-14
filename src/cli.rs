//! # Command-line interface
//!
//! The verbs the `tcal` binary offers: [`TemplateCommand`] prints the TOML form
//! of a calendar, [`EditCommand`] edits one through `$EDITOR`,
//! [`ApplyCommand`] folds an edited form back, and [`MergeCommand`] decides
//! what a three-way merge could not.
//!
//! [`Cli`] is the clap entry point parsed by main and [`Command`] the flat
//! grammar it dispatches to, one module per verb below it. [`args`] holds what
//! several verbs take, [`editor`] the round trip through `$EDITOR`, [`output`]
//! what the verbs print, [`error`] the codes of their failures and
//! [`json_schema`] the schemas of both.
//!
//! A source resolves deterministically: `-` reads stdin, an existing file is
//! read, otherwise the value is treated as literal iCalendar contents, and
//! omitting it starts from a blank template. The paths back to an iCalendar
//! are `edit` and `apply`, where the calendar the form came from is still in
//! hand.

pub mod apply;
pub mod args;
pub mod edit;
pub mod editor;
pub mod error;
pub mod json_schema;
pub mod merge;
pub mod output;
pub mod template;

use alloc::format;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use pimalaya_cli::{
    clap::{
        args::{JsonFlag, LogFlags},
        commands::{
            completion::CompletionCommand, json_schema::JsonSchemaCommand, manual::ManualCommand,
        },
    },
    footer, long_version,
    printer::Printer,
};

use crate::cli::{
    apply::ApplyCommand, edit::EditCommand, merge::MergeCommand, template::TemplateCommand,
};

/// The tCal command-line interface.
#[derive(Parser, Debug)]
#[command(name = env!("CARGO_PKG_NAME"))]
#[command(author, version, about)]
#[command(long_version = long_version!())]
#[command(after_help = footer!())]
#[command(propagate_version = true, infer_subcommands = true)]
pub struct Cli {
    /// The verb to run.
    #[command(subcommand)]
    pub cmd: Command,
    /// Whether output and errors are rendered as JSON rather than as text.
    #[command(flatten)]
    pub json: JsonFlag,
    /// Where and how verbosely the logger writes.
    #[command(flatten)]
    pub log: LogFlags,
}

/// The verbs tCal exposes.
///
/// A variant carries no documentation of its own: it delegates to the command
/// type it wraps, whose docs are what clap renders as that subcommand's help.
#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(visible_alias = "tpl")]
    Template(TemplateCommand),
    Edit(EditCommand),
    Apply(ApplyCommand),
    Merge(MergeCommand),
    #[command(alias = "completions")]
    Completion(CompletionCommand),
    #[command(alias = "manuals")]
    Manual(ManualCommand),
    #[command(alias = "json-schemas")]
    JsonSchema(JsonSchemaCommand),
}

impl Command {
    /// Runs the parsed verb, a library failure carrying its code.
    pub fn execute(self, printer: &mut impl Printer) -> Result<()> {
        let result = match self {
            Self::Template(cmd) => cmd.execute(printer),
            Self::Apply(cmd) => cmd.execute(printer),
            Self::Edit(cmd) => cmd.execute(printer),
            Self::Merge(cmd) => cmd.execute(printer),
            Self::Completion(cmd) => cmd.execute(printer, Cli::command()),
            Self::Manual(cmd) => cmd.execute(printer, Cli::command()),
            Self::JsonSchema(cmd) => cmd.execute(printer, json_schema::registry()),
        };

        result.map_err(error::coded)
    }
}

#[cfg(test)]
mod tests {
    use alloc::{borrow::ToOwned, string::String, vec::Vec};

    use clap::CommandFactory;

    use crate::cli::Cli;

    /// The verbs are spelled the way every document spells them.
    ///
    /// clap derives a subcommand's name from its variant, so a variant
    /// carrying the library's `Tcal` prefix would offer `tcal-template` and
    /// `tcal-merge`, which is neither what the README documents nor what
    /// `infer_subcommands` would reach from `template`.
    #[test]
    fn a_verb_is_named_after_itself() {
        let names: Vec<String> = Cli::command()
            .get_subcommands()
            .map(|cmd| cmd.get_name().to_owned())
            .collect();

        for verb in ["template", "edit", "merge"] {
            assert!(names.contains(&verb.to_owned()), "{names:?}");
        }
    }
}
