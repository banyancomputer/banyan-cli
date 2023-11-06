//! this crate is the binary for the tomb project. It contains the main function and the command line interface.
use anyhow::Result;
use banyan_cli::banyan_cli::cli::{self, commands::RunnableCommand};
use clap::Parser;
use std::io::Write;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments. see args.rs
    let cli = cli::args::Args::parse();

    // TODO eventually make options to format it differently?
    std::env::set_var("RUST_LOG", "info");
    env_logger::Builder::new()
        .filter_level(cli.verbose.into())
        .format(|buf, record| writeln!(buf, "[{}] {}", record.level(), record.args()))
        .format_timestamp(None)
        .format_level(true)
        .format_module_path(false)
        .init();

    // Determine the command being executed run appropriate subcommand
    let _ = cli.command.run().await;

    Ok(())
}
