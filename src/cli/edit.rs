//! # Edit command
//!
//! The full round trip: project a calendar as TOML, open `$EDITOR` on it, fold
//! the edits back, and emit the resulting iCalendar.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{clap::parsers::path_parser, printer::Printer};

use crate::{
    cli::{
        args::{ComponentFlags, EditorArg, Output, SourceArg},
        editor::Editor,
    },
    template::TcalTemplate,
};

/// Edit an iCalendar as TOML in `$EDITOR`, blank or prefilled from a source.
///
/// The calendar is projected as a fillable TOML form, opened in an editor,
/// then folded back onto the source. Only the lines you changed are
/// re-rendered, so every other byte survives, the properties and component
/// types this form does not show included.
#[derive(Debug, Parser)]
pub struct EditCommand {
    /// The iCalendar to prefill the form from, and to fold the edits onto.
    #[command(flatten)]
    pub source: SourceArg,
    /// The component types the form shows, and the only ones it reconciles.
    #[command(flatten)]
    pub components: ComponentFlags,
    /// The editor the form is opened in.
    #[command(flatten)]
    pub editor: EditorArg,
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
        let source = self.source.load()?;
        let template = TcalTemplate::parse(&source)?.with_types(&self.components.selected())?;

        let projected = template.project();
        let editor = Editor {
            document: &projected,
            command: self.editor.editor.as_deref(),
        };

        let out = editor.apply(printer, |edited| template.apply(edited))?;

        let target = self.output.or_else(|| self.source.file_path());
        Output(target.as_deref()).write(out.as_bytes())
    }
}
