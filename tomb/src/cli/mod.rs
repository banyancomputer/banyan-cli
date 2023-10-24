/// Arguments consist of Command an Verbosity
pub mod args;
/// Commands to run
pub mod commands;
/// Ways of specifying resources
pub mod specifiers;
/// Debug level
pub mod verbosity;

#[cfg(test)]
mod test {
    use super::commands::*;
    use crate::{cli::specifiers::*, types::config::globalconfig::GlobalConfig, utils::test::*};
    use anyhow::{anyhow, Result};
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

    async fn cmd_delete(origin: &Path) -> Result<()> {
        let mut global = GlobalConfig::from_disk().await?;
        let local = global.get_bucket(origin).ok_or(anyhow!("no bucket"))?;
        local.remove_data()?;
        // Find index of bucket
        let index = global
            .buckets
            .iter()
            .position(|b| b == &local)
            .expect("cannot find index in buckets");
        // Remove bucket config from global config
        global.buckets.remove(index);
        global.to_disk()?;
        Ok(())
    }

    // Run the Prepare pipeline through the CLI
    fn cmd_prepare(origin: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Prepare {
                bucket_specifier: BucketSpecifier::with_origin(origin),
                follow_links: true,
            },
        }
    }

    // Run the Restore pipeline through the CLI
    fn cmd_restore(origin: &Path, restored: &Path) -> TombCommand {
        TombCommand::Buckets {
            command: BucketsCommand::Restore {
                bucket_specifier: BucketSpecifier::with_origin(origin),
                restore_path: Some(restored.to_path_buf()),
            },
        }
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "cli_init";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Deinitialize for user
        cmd_delete(origin).await.ok();
        // Assert failure
        assert!(cmd_prepare(origin).run().await.is_err());
        // Initialization worked
        cmd_create(origin).run().await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket(origin)
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
        // Deinit if present
        cmd_delete(origin).await.ok();
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket(origin)
            .is_none());
        // Initialization worked
        cmd_create(origin).run().await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket(origin)
            .is_some());
        // Deinitialize the directory
        cmd_delete(origin).await?;
        // Assert the bucket is gone again
        assert!(GlobalConfig::from_disk()
            .await?
            .get_bucket(origin)
            .is_none());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn prepare() -> Result<()> {
        let test_name = "cli_prepare";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        cmd_create(origin).run().await?;
        // Run prepare and assert success
        cmd_prepare(origin).run().await?;
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn restore() -> Result<()> {
        let test_name = "cli_restore";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        cmd_create(origin).run().await?;
        // Run prepare and assert success
        cmd_prepare(origin).run().await?;
        // Create restored dir
        let restored = &origin
            .parent()
            .expect("origin has no parent")
            .join("restored");
        create_dir(restored).ok();
        // Run restore and assert success
        cmd_restore(origin, restored).run().await?;
        // Assert equality
        assert_paths(origin, restored).expect("restored dir does not match origin");
        // Teardown test
        test_teardown(test_name).await
    }
}
