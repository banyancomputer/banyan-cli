use crate::daemon::start_daemon;
use async_trait::async_trait;
use banyan_guts::cli::commands::RunnableCommand;
use banyan_guts::cli::verbosity::MyVerbosity;
use banyan_guts::native::NativeError;
use clap::{command, Parser, Subcommand};

#[derive(Debug, Subcommand, Clone)]
pub enum DaemonCommand {
    /// Start the daemon
    Start,
    /// Stop the daemon
    Stop,
    /// Restart the daemon
    Restart,
    /// Reload the daemon
    Reload,
    /// Get the status of the daemon
    Status,
    /// Get the version of the daemon
    Version,
}

#[async_trait(?Send)]
impl RunnableCommand<NativeError> for DaemonCommand {
    // TODO: implement nativeerror for daemon, or do something else
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            DaemonCommand::Start => {
                start_daemon().await?;
                // TODO make this nicer
                Ok("you started it :) :)".to_string())
            }
            _ => Err(NativeError::daemon_error("Not implemented".to_string())),
        }
    }
}

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: DaemonCommand,
    /// Verbosity level.
    #[arg(
        short,
        long,
        help = "logging verbosity level",
        default_value = "normal"
    )]
    pub verbose: MyVerbosity,
}
