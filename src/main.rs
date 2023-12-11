//! this crate is the binary for the tomb project. It contains the main function and the command line interface.
#[cfg(target_arch = "wasm32")]
fn main() {
    panic!("there is no main in wasm!");
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
use {
    banyan_cli::{
        self,
        cli::{args::Args, commands::RunnableCommand},
    },
    clap::Parser,
    tracing::Level,
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer},
};

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "cli")]
#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let cli = Args::parse();

    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    // Determine the command being executed run appropriate subcommand
    let _ = cli.command.run().await;
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "cli"))]
#[tokio::main]
async fn main() {
    println!("Enable the CLI feature to interact with the CLI");
}
