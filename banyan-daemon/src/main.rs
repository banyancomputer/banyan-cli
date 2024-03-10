use tracing::Level;
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::Layer;

mod service;

use banyan_guts::cli2::verbosity::MyVerbosity;
use clap::{command, Parser};

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Verbosity level.
    #[arg(
        short,
        long,
        help = "logging verbosity level",
        default_value = "normal"
    )]
    pub verbose: MyVerbosity,
}

#[tokio::main]
async fn main() {
    // Parse command line arguments. see args.rs
    let _cli = Args::parse();

    let (non_blocking_writer, _guard) = tracing_appender::non_blocking(std::io::stderr());
    let env_filter = EnvFilter::builder()
        .with_default_directive(Level::INFO.into())
        .from_env_lossy();

    // TODO: something with verbosity

    let stderr_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .with_target(false)
        .with_file(false)
        .with_line_number(false)
        .with_writer(non_blocking_writer)
        .with_filter(env_filter);

    tracing_subscriber::registry().with(stderr_layer).init();

    crate::service::daemonize_self().unwrap();
}
