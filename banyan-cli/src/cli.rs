// mod daemon;

// use std::io::Read;

// use crate::native::NativeError;
// use banyan_guts::cli2::commands::account::AccountCommand;
// use async_trait::async_trait;
// use clap::Subcommand;
// pub use sync::SyncCommand;
// pub use runnable_command::RunnableCommand;
// pub use serde::{Deserialize, Serialize};

// use self::daemon::DaemonCommand;

// use std::io::Read;

use async_trait::async_trait;
use banyan_guts::cli2::commands::AccountCommand;
use banyan_guts::cli2::commands::BanyanServiceApiCommand;
use banyan_guts::cli2::commands::RunnableCommand;
use banyan_guts::cli2::commands::SyncCommand;
use banyan_guts::cli2::verbosity::MyVerbosity;

use banyan_guts::native::NativeError;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// use tracing::info;

/// Prompt the user for a y/n answer
// pub fn prompt_for_bool(msg: &str) -> bool {
//     info!("{msg} y/n");
//     loop {
//         let mut input = [0];
//         let _ = std::io::stdin().read(&mut input);
//         match input[0] as char {
//             'y' | 'Y' => return true,
//             'n' | 'N' => return false,
//             _ => info!("y/n only please."),
//         }
//     }
// }

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: BanyanCliCommand,
    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub verbose: MyVerbosity,
}

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone, Serialize, Deserialize)]
pub enum BanyanCliCommand {
    /// Manage daemon
    Daemon {
        /// Subcommand
        #[clap(subcommand)]
        command: DaemonCommand,
    },
    /// Account Login and Details
    Account {
        /// Subcommand
        #[clap(subcommand)]
        command: AccountCommand,
    },
    /// Sync management
    Sync {
        /// Subcommand
        #[clap(subcommand)]
        command: SyncCommand,
    },
}

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
                // check that the banyan daemon isn't running
                if !crate::daemon::daemon_is_running() {
                    // start the daemon
                    crate::daemon::start_daemon()?;
                    Ok("Started daemon".to_string())
                } else {
                    Err(NativeError::custom_error(
                        "Banyan daemon is already running",
                    ))
                }
            }
            DaemonCommand::Stop => {
                if !crate::daemon::daemon_is_running() {
                    Err(NativeError::custom_error("Banyan daemon is not running"))
                } else {
                    // stop the daemon
                    crate::daemon::stop_daemon()?;
                    Ok("Stopped daemon".to_string())
                }
            }
            DaemonCommand::Restart => {
                if !crate::daemon::daemon_is_running() {
                    // start the daemon
                    crate::daemon::start_daemon()?;
                    Ok("Started daemon".to_string())
                } else {
                    // stop the daemon
                    crate::daemon::stop_daemon()?;
                    // start the daemon
                    crate::daemon::start_daemon()?;
                    Ok("Restarted daemon".to_string())
                }
            }
            DaemonCommand::Status => {
                if crate::daemon::daemon_is_running() {
                    Ok("Daemon is running".to_string())
                } else {
                    Ok("Daemon is not running".to_string())
                }
            }
            DaemonCommand::Version => {
                unimplemented!();
            }
        }
    }
}

#[async_trait]
impl RunnableCommand<NativeError> for BanyanCliCommand {
    async fn run_internal(self) -> Result<String, NativeError> {
        match self {
            BanyanCliCommand::Daemon { command } => Ok(command.run_internal().await?),
            BanyanCliCommand::Account { command } => Ok(command.run_internal().await?),
            BanyanCliCommand::Sync { command } => Ok(command.run_internal().await?),
        }
    }
}

impl TryInto<BanyanServiceApiCommand> for BanyanCliCommand {
    type Error = NativeError;

    fn try_into(self) -> Result<BanyanServiceApiCommand, Self::Error> {
        match self {
            BanyanCliCommand::Sync { command } => Ok(BanyanServiceApiCommand::Sync { command }),
            BanyanCliCommand::Account { command } => {
                Ok(BanyanServiceApiCommand::Account { command })
            }
            BanyanCliCommand::Daemon { .. } => Err(NativeError::custom_error(
                "you can't run a daemon command in the daemon.",
            )),
        }
    }
}
