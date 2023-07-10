pub mod command;
pub mod args;
pub mod verbosity;

use anyhow::Result;
use std::env::current_dir;
use crate::pipelines::{pack, configure, unpack, pull, push, add, remove};
use command::{Command, ConfigSubCommand};

// TODO add support for https://docs.rs/keyring/latest/keyring/
// TODO what's going on with buckets? these are URLs right?
pub async fn run(command: Command) -> Result<()> {
    // Determine the command being executed
    match command {
        // Execute the packing command
        Command::Pack {
            origin,
            follow_links,
        } => {
            if let Some(origin) = origin {
                pack::pipeline(&origin, follow_links).await?;
            } else {
                pack::pipeline(&current_dir()?, follow_links).await?;
            }
        }
        // Execute the unpacking command
        Command::Unpack {
            origin,
            unpacked,
        } => {
            if let Some(origin) = origin {
                unpack::pipeline(&origin, &unpacked).await?;
            } else {
                unpack::pipeline(&current_dir()?, &unpacked).await?;
            }
        }
        Command::Init {
            dir
        } => {
            // Initialize here
            if let Some(dir) = dir {
                configure::init(&dir)?;
            }
            else {
                configure::init(&current_dir()?)?;
            }
        },
        Command::Deinit {
            dir
        } => {
            // Initialize here
            if let Some(dir) = dir {
                configure::deinit(&dir)?;
            }
            else {
                configure::deinit(&current_dir()?)?;
            }
        },
        Command::Login => unimplemented!("todo... a little script where you log in to the remote and enter your api key. just ends if you're authenticated. always does an auth check. little green checkmark :D."),
        Command::Register { bucket_name: _ } =>
            unimplemented!("todo... register a bucket on the remote. should create a database entry on the remote. let alex know we need one more api call for this."),
        Command::Configure { subcommand } => {
            match subcommand {
                ConfigSubCommand::SetRemote { address } => {
                    configure::remote(&address)?;
                }
            }
        },
        Command::Daemon => unimplemented!("todo... omg fun... cronjob"),
        Command::Pull {
            dir
        } => {
            // Start the Pull pipeline
            pull::pipeline(&dir).await?;
        },
        Command::Push {
            dir,
        } => {
            // Start the Push pipeline
            push::pipeline(&dir).await?;
        },
        Command::Add { origin, input_file, wnfs_path } => {
            add::pipeline(&origin, &input_file, &wnfs_path).await?;
        },
        Command::Remove { origin, wnfs_path } => {
            remove::pipeline(&origin, &wnfs_path).await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{types::config::globalconfig::GlobalConfig, utils::test::*, cli::run};
    use anyhow::Result;
    use dir_assert::assert_paths;
    use fs_extra::file;
    use serial_test::serial;
    use std::{
        fs::{create_dir, metadata},
        path::Path
    };
    use super::command::{Command, ConfigSubCommand};

    fn cmd_init(dir: &Path) -> Command {
        Command::Init { dir: Some(dir.to_path_buf()) }
    }

    fn cmd_deinit(dir: &Path) -> Command {
        Command::Deinit { dir: Some(dir.to_path_buf()) }
    }

    fn cmd_configure_remote(address: &str) -> Command {
        Command::Configure { subcommand: ConfigSubCommand::SetRemote { address: address.to_string() } }
    }

    // Run the Pack pipeline through the CLI
    fn cmd_pack(origin: &Path) -> Command {
        Command::Pack { origin: Some(origin.to_path_buf()), follow_links: true }
    }

    // Run the Unpack pipeline through the CLI
    fn cmd_unpack(origin: &Path, unpacked: &Path) -> Command {
        Command::Unpack { origin: Some(origin.to_path_buf()), unpacked: unpacked.to_path_buf() }
    }

    // Run the Push pipeline through the CLI
    fn cmd_push(dir: &Path) -> Command {
        Command::Push { dir: dir.to_path_buf() }
    }

    // Run the Pull pipeline through the CLI
    fn cmd_pull(dir: &Path) -> Command {
        Command::Pull { dir: dir.to_path_buf() }
    }

    #[tokio::test]
    #[serial]
    async fn init() -> Result<()> {
        let test_name = "init";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Initialization worked
        run(cmd_init(&origin)).await?;
        // Assert the bucket exists now
        let global = GlobalConfig::from_disk()?;
        // Assert that there is always a wrapping key
        assert!(global.wrapping_key_from_disk().is_ok());
        let bucket = global.get_bucket(origin);
        assert!(bucket.is_some());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn init_deinit() -> Result<()> {
        let test_name = "init_deinit";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Initialization worked
        run(cmd_init(origin)).await?;
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_some());
        // Deinitialize the directory
        run(cmd_deinit(origin)).await?;
        // Assert the bucket is gone again
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn configure_remote() -> Result<()> {
        let test_name = "configure_remote";
        // Setup test
        let input_dir = &test_setup(test_name).await?;

        // Initialize
        run(cmd_init(&input_dir)).await?;

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
    async fn pack() -> Result<()> {
        let test_name = "pack";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_init(origin)).await?;
        // Run pack and assert success
        run(cmd_pack(origin)).await?;
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn unpack() -> Result<()> {
        let test_name = "unpack";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_init(origin)).await?;
        // Run pack and assert success
        run(cmd_pack(origin)).await?;
        // Create unpacked dir
        let unpacked = &origin.parent().unwrap().join("unpacked");
        create_dir(unpacked).ok();
        // Run unpack and assert success
        run(cmd_unpack(origin, unpacked)).await?;
        // Assert equality
        assert_paths(origin, unpacked).unwrap();
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn push_pull() -> Result<()> {
        let test_name = "push_pull";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        run(cmd_init(origin)).await?;
        // Configure remote endpoint
        run(cmd_configure_remote("http://127.0.0.1:5001")).await?;
        // Run pack locally and assert success
        run(cmd_pack(origin)).await?;

        let v1_path = &GlobalConfig::from_disk()?
            .get_bucket(origin)
            .unwrap()
            .content
            .path;
        let v1_moved = &v1_path.parent().unwrap().join("old_content.car");
        file::move_file(v1_path, v1_moved, &file::CopyOptions::new())?;

        // Run push and assert success
        run(cmd_push(origin)).await?;
        // Run unpack and assert success
        run(cmd_pull(origin)).await?;
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(metadata(v1_path)?.len(), metadata(v1_moved)?.len(),);
        // Teardown test
        test_teardown(test_name).await
    }
}
