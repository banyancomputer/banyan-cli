/// Arguments consist of Command an Verbosity
pub mod args;
/// Commands to run
pub mod commands;
/// Ways of specifying resources
pub mod specifiers;
/// Debug level
pub mod verbosity;

use crate::pipelines::banyan_api::*;
use anyhow::Result;
use commands::TombCommand;

/// Based on the Command, run pipelines
pub async fn run(command: TombCommand) -> Result<()> {
    // Determine the command being executed run appropriate subcommand
    let result: Result<String, anyhow::Error> = match command {
        TombCommand::Api {
            command: subcommand,
        } => configure::pipeline(subcommand).await,
        TombCommand::Account {
            command: subcommand,
        } => auth::pipeline(subcommand).await,
        TombCommand::Buckets {
            command: subcommand,
        } => bucket::pipeline(subcommand).await,
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

#[cfg(test)]
mod test {
    use super::commands::*;
    use crate::{
        cli::{run, specifiers::*},
        types::config::globalconfig::GlobalConfig,
        utils::test::*,
    };
    use anyhow::Result;
    use dir_assert::assert_paths;
    use serial_test::serial;
    use std::{fs::create_dir, path::Path};

    #[allow(dead_code)]
    #[cfg(feature = "fake")]
    fn cmd_register() -> TombCommand {
        TombCommand::Account {
            command: AccountCommand::Register,
        }
    }

    fn cmd_create(origin: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Create {
                name: "Bucket Name".to_string(),
                origin: Some(origin.to_path_buf()),
            },
        }
    }

    fn cmd_delete(origin: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Delete(BucketSpecifier::with_origin(origin)),
        }
    }

    // Run the Bundle pipeline through the CLI
    fn cmd_bundle(origin: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Bundle {
                bucket_specifier: BucketSpecifier::with_origin(origin),
                follow_links: true,
            },
        }
    }

    // Run the Extract pipeline through the CLI
    fn cmd_extract(origin: &Path, extracted: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Extract {
                bucket_specifier: BucketSpecifier::with_origin(origin),
                output: extracted.to_path_buf(),
            },
        }
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "cli_init";
        // Setup test
        let (origin, _) = &test_setup(test_name).await?;
        // Deinitialize for user
        run(cmd_delete(origin)).await.ok();
        // Assert failure
        assert!(run(cmd_bundle(origin)).await.is_err());
        // Initialization worked
        run(cmd_create(origin)).await?;
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
        let (origin, _) = &test_setup(test_name).await?;
        // Deinit if present
        run(cmd_delete(origin)).await.ok();
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_none());
        // Initialization worked
        run(cmd_create(origin)).await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket_by_origin(origin)
            .is_some());
        // Deinitialize the directory
        run(cmd_delete(origin)).await?;
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
        let (origin, _) = &test_setup(test_name).await?;
        // Initialize
        run(cmd_create(origin)).await?;
        // Configure remote endpoint
        // run(cmd_configure_remote("http://127.0.0.1:5001")).await?;
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn bundle() -> Result<()> {
        let test_name = "cli_bundle";
        // Setup test
        let (origin, _) = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_create(origin)).await?;
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
        let (origin, _) = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_create(origin)).await?;
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
}
