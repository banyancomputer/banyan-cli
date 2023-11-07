//! this crate is the binary for the tomb project. It contains the main function and the command line interface.
#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("there is no main in wasm!");
}

#[cfg(not(target_arch = "wasm32"))]
use anyhow::Result;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
use {
    banyan::{
        self,
        banyan_cli::{args::Args, commands::RunnableCommand},
    },
    clap::Parser,
    std::io::Write,
};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
#[tokio::main]
pub async fn main() -> Result<()> {
    // Parse command line arguments. see args.rs
    let cli = Args::parse();

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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "cli"))]
#[tokio::main]
pub async fn main() -> Result<()> {
    println!("Enable the CLI feature to interact with the CLI");
    Ok(())
}
