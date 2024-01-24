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
    #[cfg(feature = "integration-tests")]
    use crate::cli::commands::AccountCommand;
    use crate::{
        cli::{
            commands::{DrivesCommand, RunnableCommand, TombCommand},
            specifiers::DriveSpecifier,
        },
        native::{configuration::globalconfig::GlobalConfig, NativeError},
        utils::{
            testing::local_operations::{test_setup, test_teardown},
            UtilityError,
        },
    };
    use serial_test::serial;
    use std::path::Path;

    #[allow(dead_code)]
    #[cfg(feature = "integration-tests")]
    fn cmd_register() -> TombCommand {
        TombCommand::Account {
            command: AccountCommand::Register,
        }
    }

    fn cmd_create(origin: &Path) -> TombCommand {
        TombCommand::Drives {
            command: DrivesCommand::Create {
                name: "Bucket Name".to_string(),
                origin: Some(origin.to_path_buf()),
            },
        }
    }

    async fn cmd_delete(origin: &Path) -> Result<(), UtilityError> {
        let mut global = GlobalConfig::from_disk().await?;
        let local = global
            .get_bucket(origin)
            .ok_or(NativeError::missing_local_drive())?;
        global.remove_bucket(&local)?;
        Ok(())
    }

    // Run the Prepare pipeline through the CLI
    fn cmd_prepare(origin: &Path) -> TombCommand {
        TombCommand::Drives {
            command: DrivesCommand::Prepare {
                drive_specifier: DriveSpecifier::with_origin(origin),
                follow_links: true,
            },
        }
    }

    // Run the Restore pipeline through the CLI
    fn cmd_restore(origin: &Path) -> TombCommand {
        TombCommand::Drives {
            command: DrivesCommand::Restore {
                drive_specifier: DriveSpecifier::with_origin(origin),
            },
        }
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<(), UtilityError> {
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
    async fn init_deinit() -> Result<(), UtilityError> {
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
    async fn prepare() -> Result<(), UtilityError> {
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
    async fn restore() -> Result<(), UtilityError> {
        let test_name = "cli_restore";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        cmd_create(origin).run().await?;
        // Run prepare and assert success
        cmd_prepare(origin).run().await?;
        // Run restore and assert success
        cmd_restore(origin).run().await?;
        // Assert equality
        // let restored = GlobalConfig::from_disk().await?.get_bucket(origin)?.origin;
        // assert_paths(origin, rest÷ored).expect("restored dir does not match origin");
        // Teardown test
        test_teardown(test_name).await
    }
}
