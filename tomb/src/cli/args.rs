use clap::{command, Parser};

use super::{command::Command, verbosity::MyVerbosity};

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: Command,
    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub verbose: MyVerbosity,
}
