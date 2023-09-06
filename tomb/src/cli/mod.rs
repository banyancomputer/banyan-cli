/// Arguments consist of Command an Verbosity
pub mod args;
/// Command to run
pub mod command;
/// Debug level
pub mod verbosity;

use crate::pipelines::{banyan_api::*, *};
use anyhow::Result;
use command::Command;

/// Based on the Command, run pipelines
pub async fn run(command: Command) -> Result<()> {
    // Determine the command being executed run appropriate subcommand
    let result: Result<String, anyhow::Error> = match command {
        Command::SetRemote { address } => configure::remote(&address).await,
        Command::Auth { subcommand } => auth::pipeline(subcommand).await,
        Command::Buckets { subcommand } => bucket::pipeline(subcommand).await,
    };

    // Provide output based on that
    match result {
        Ok(message) => {
            println!("{}", message);
            Ok(())
        }
        Err(error) => {
            println!("{}", error);
            Err(error)
        }
    }
}

/*
#[cfg(test)]
mod test {
    use super::command::Command;
    use crate::{cli::run, types::config::globalconfig::GlobalConfig, utils::test::*};
    use anyhow::Result;
    use dir_assert::assert_paths;
    use serial_test::serial;
    use std::{fs::create_dir, path::Path};

    fn cmd_configure_remote(address: &str) -> Command {
        Command::SetRemote {
            address: address.to_string(),
        }
    }

    fn cmd_init(dir: &Path) -> Command {
        Command::Init {
            dir: Some(dir.to_path_buf()),
        }
    }

    fn cmd_deinit(dir: &Path) -> Command {
        Command::Deinit {
            dir: Some(dir.to_path_buf()),
        }
    }


    // Run the Bundle pipeline through the CLI
    fn cmd_bundle(origin: &Path) -> Command {
        Command::Bundle {
            origin: Some(origin.to_path_buf()),
            follow_links: true,
        }
    }

    // Run the Extract pipeline through the CLI
    fn cmd_extract(origin: &Path, extracted: &Path) -> Command {
        Command::Extract {
            origin: Some(origin.to_path_buf()),
            extracted: extracted.to_path_buf(),
        }
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "cli_init";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Deinitialize for user
        run(cmd_deinit(origin)).await?;
        // Assert failure
        assert!(run(cmd_bundle(origin)).await.is_err());
        // Initialization worked
        run(cmd_init(origin)).await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_some());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn init_deinit() -> Result<()> {
        let test_name = "cli_init_deinit";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_none());
        // Initialization worked
        run(cmd_init(origin)).await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_some());
        // Deinitialize the directory
        run(cmd_deinit(origin)).await?;
        // Assert the bucket is gone again
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_none());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn configure_remote() -> Result<()> {
        let test_name = "cli_configure_remote";
        // Setup test
        let input_dir = &test_setup(test_name).await?;

        // Initialize
        run(cmd_init(input_dir)).await?;

        // Configure remote endpoint
        run(cmd_configure_remote("http://127.0.0.1:5001")).await?;

        // Load the modified Manifest
        // let _manifest = manifest_from_disk(&input_dir.join(".tomb"))?;
        // Expect that the remote endpoint was successfully updated
        // assert_eq!(manifest.cold_remote.addr, "http://127.0.0.1:5001");
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn bundle() -> Result<()> {
        let test_name = "cli_bundle";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_init(origin)).await?;
        // Run bundle and assert success
        run(cmd_bundle(origin)).await?;
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn extract() -> Result<()> {
        let test_name = "cli_extract";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_init(origin)).await?;
        // Run bundle and assert success
        run(cmd_bundle(origin)).await?;
        // Create extracted dir
        let extracted = &origin
            .parent()
            .expect("origin has no parent")
            .join("extracted");
        create_dir(extracted).ok();
        // Run extract and assert success
        run(cmd_extract(origin, extracted)).await?;
        // Assert equality
        assert_paths(origin, extracted).expect("extracted dir does not match origin");
        // Teardown test
        test_teardown(test_name).await
    }
} */
