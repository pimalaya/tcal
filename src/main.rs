//! # tCal
//!
//! Binary entry point of the tCal CLI. It parses the command-line interface
//! declared in [`tcal::cli`], wires the logger and the printer from the shared
//! pimalaya-cli toolkit, then hands control to the parsed command.
//!
//! A command line clap refuses exits 2, a failure prints the failure object,
//! its code when it has one, and exits 1.
//!
//! Everything below this file lives in the library, whose own header carries
//! the crate architecture: see [`tcal`].

use anyhow::Result;
use pimalaya_cli::{clap::parse, error::ErrorReport, log::Logger, printer::StdoutPrinter};

use tcal::cli::{Cli, error::ErrorCode};

fn main() {
    let cli = parse::<Cli>();
    let mut printer = StdoutPrinter::new(&cli.json);

    let result = execute(cli, &mut printer);
    ErrorReport::<ErrorCode>::eval_coded(&mut printer, result);
}

/// Initializes the logger, then runs the parsed command.
fn execute(cli: Cli, printer: &mut StdoutPrinter) -> Result<()> {
    Logger::try_init(&cli.log)?;
    cli.cmd.execute(printer)
}
