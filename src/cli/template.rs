//! # Template command
//!
//! Printing the TOML form of a calendar, blank or prefilled. It always emits
//! TOML and never an iCalendar, the way back being the edit verb.

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use pimalaya_cli::{clap::parsers::path_parser, printer::Printer};

use crate::{
    cli::args::{ComponentFlags, Output, SourceArg},
    template::TcalTemplate,
};

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
        let source = self.source.load()?;
        let toml = TcalTemplate::parse(&source)?
            .with_types(&self.components.selected())?
            .project();

        Output(self.output.as_deref()).write(toml.as_bytes())
    }
}
