use async_trait::async_trait;
use clap::Subcommand;

use banyan_guts::cli2::commands::RunnableCommand;
use banyan_guts::native::NativeError;

use serde::{Deserialize, Serialize};

/// Subcommand for daemon
#[derive(Subcommand, Clone, Debug, Serialize, Deserialize)]
pub enum DaemonCommand {
    /// Start Banyan daemon
    Start,
    /// Stop Banyan daemon
    Stop,
    /// Restart Banyan daemon
    Restart,
    /// Get the status of the daemon
    Status,
    /// Get the version of the daemon
    Version,
}

#[async_trait]
impl RunnableCommand<NativeError> for DaemonCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            DaemonCommand::Start => {
                unimplemented!()
            }
            DaemonCommand::Stop => {
                unimplemented!()
            }
            DaemonCommand::Restart => {
                unimplemented!()
            }
            // TODO graceful kill
            DaemonCommand::Status => {
                unimplemented!()
            }
            DaemonCommand::Version => {
                unimplemented!()
            }
        }
    }
}
