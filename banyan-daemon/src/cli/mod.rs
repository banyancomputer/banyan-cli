use std::path::PathBuf;

use async_trait::async_trait;
use banyan_guts::cli2::commands::RunnableCommand;
use banyan_guts::cli2::verbosity::MyVerbosity;
use banyan_guts::native::NativeError;
use clap::{command, Parser, Subcommand};
use config::Config;

use self::daemon_util::{build_daemonize, daemon_is_running, kill_daemon};

mod daemon_util;

#[derive(Debug, Subcommand, Clone)]
pub enum DaemonCommand {
    /// Start the daemon
    Start,
    /// Stop the daemon- currently just hits it with a SIGKILL lol. maybe think about that
    Stop,
    /// Restart the daemon
    Restart,
    /// Get the status of the daemon
    Status,
    /// Get the version of the daemon
    Version,
}

#[async_trait]
impl RunnableCommand<NativeError> for DaemonCommand {
    // TODO: implement nativeerror for daemon, or do something else
    async fn run_internal(self) -> Result<String, NativeError> {
        let _settings = Config::builder()
            // Add in `/etc/banyan/Banyan.toml`
            .add_source(config::File::with_name("/etc/banyan/Banyan"))
            // Add in settings from the environment (with a prefix of APP)
            // Eg.. `APP_DEBUG=1 ./target/app` would set the `debug` key
            .add_source(config::Environment::with_prefix("BANYAN"))
            .build()
            .unwrap();

        // Ensure /etc/banyan exists, if not create it
        // this is... not platform-agnostic
        let etc_banyan_path = PathBuf::from("/etc/banyan");
        if !etc_banyan_path.exists() {
            std::fs::create_dir_all(&etc_banyan_path)
                .expect("Failed to create /etc/banyan directory");
        }

        match self {
            DaemonCommand::Start => {
                if daemon_is_running() {
                    return Err(NativeError::daemon_error(
                        "Daemon is already running".to_string(),
                    ));
                };
                build_daemonize()
            }
            DaemonCommand::Restart => {
                if !daemon_is_running() {
                    return Err(NativeError::daemon_error(
                        "Daemon is not running".to_string(),
                    ));
                };
                build_daemonize()
            }
            // TODO graceful kill
            DaemonCommand::Stop => {
                if !daemon_is_running() {
                    return Err(NativeError::daemon_error(
                        "Daemon is not running".to_string(),
                    ));
                };
                kill_daemon()
            }
            DaemonCommand::Status => {
                if daemon_is_running() {
                    Ok("Daemon is running".to_string())
                } else {
                    Ok("Daemon is not running".to_string())
                }
            }
            // TODO automate me nicer
            DaemonCommand::Version => Ok("0.0.1".to_string()),
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
