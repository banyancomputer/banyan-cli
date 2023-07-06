use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;

// TODO add support for https://docs.rs/keyring/latest/keyring/
// TODO what's going on with buckets? these are URLs right?

/// Sub-commands associated with configuration
#[derive(Subcommand, Clone, Debug)]
pub enum ConfigSubCommands {
    /// Set the remote endpoint where buckets are synced to / from
    SetRemote {
        /// Server address
        #[arg(short, long, help = "full server address")]
        address: String,
    },
}

//
/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub enum Commands {
    /// Packing a filesystem on disk into an encrypted WNFS CAR file
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        origin: Option<PathBuf>,

        // /// Maximum size for each chunk, defaults to 1GiB.
        // #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        // chunk_size: u64,
        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    /// Reconstructing a filesystem from an encrypted WNFS CAR file
    Unpack {
        /// Origin path
        #[arg(short, long, help = "path to original filesystem")]
        origin: Option<PathBuf>,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory for filesystem reconstruction")]
        unpacked: PathBuf,
    },
    /// Add an individual file or folder to an existing bucket
    Add {
        /// Origin path
        #[arg(short, long, help = "original input directory")]
        origin: PathBuf,

        /// Path of file / folder being added
        #[arg(short, long, help = "new file / directory")]
        input_file: PathBuf,

        /// Path at which the node will be added in the WNFS
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// Remove an individual file or folder from an existing bucket
    Remove {
        /// Origin path
        #[arg(short, long, help = "original input directory")]
        origin: PathBuf,

        /// Path at which the node will be removed from the WNFS if it exists
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// Update local from the remote bucket endpoint
    Pull {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "directory")]
        dir: PathBuf,
    },
    /// tomb push <bucket_name>- Push changes to a bucket to Tombolo/filecoin
    Push {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "directory")]
        dir: PathBuf,
    },
    /// Create new bucket config for a directory
    Init {
        /// Directory to init, or PWD if None
        dir: Option<PathBuf>,
    },
    /// Remove config and packed data for a directory
    Deinit {
        /// Directory to deinit, or PWD if None
        dir: Option<PathBuf>,
    },
    /// log in to tombolo remote, basically validates that your API keys or whatever are in place. must be run before registry or anything else.
    Login,
    /// tomb register <bucket_name> - Register a new bucket on the tombolo service for this data. then you can push to it. MUST be called before push.
    Register {
        /// Name of the bucket to create
        #[arg(short, long, help = "bucket name")]
        bucket_name: String,
    },
    /// tomb config <subcommand> - Configure Tombolo
    Configure {
        /// Configuration subcommand
        #[clap(subcommand)]
        subcommand: ConfigSubCommands,
    },
    /// We don't know yet
    Daemon,
}

/// Level of verbosity in debugs
#[derive(Clone, Debug, clap::ValueEnum)]
pub enum MyVerbosity {
    /// Quiet
    Quiet,
    /// Normal
    Normal,
    /// Verbose
    Verbose,
    /// Very Verbose
    VeryVerbose,
    /// Debug
    Debug,
}

impl From<MyVerbosity> for LevelFilter {
    fn from(val: MyVerbosity) -> Self {
        match val {
            MyVerbosity::Quiet => LevelFilter::Off,
            MyVerbosity::Normal => LevelFilter::Info,
            MyVerbosity::Verbose => LevelFilter::Debug,
            MyVerbosity::VeryVerbose => LevelFilter::Trace,
            MyVerbosity::Debug => LevelFilter::Trace,
        }
    }
}

/// Arguments to tomb
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Command passed
    #[command(subcommand)]
    pub command: Commands,
    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub verbose: MyVerbosity,
}

#[cfg(test)]
mod test {
    use crate::utils::tests::*;
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use dir_assert::assert_paths;
    use fs_extra::file;
    use serial_test::serial;
    use std::{
        fs::{create_dir, metadata},
        path::Path,
        process::Command,
    };
    use tomb_common::types::config::globalconfig::GlobalConfig;

    async fn cmd_init(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("init").arg(dir);
        Ok(cmd)
    }

    async fn cmd_deinit(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("deinit").arg(dir);
        Ok(cmd)
    }

    async fn cmd_configure_remote(address: &str) -> Result<Command> {
        // configure set-remote --url http://127.0.0.1 --port 5001
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("configure")
            .arg("set-remote")
            .arg("--address")
            .arg(address);
        Ok(cmd)
    }

    // Run the Pack pipeline through the CLI
    async fn cmd_pack(origin: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pack")
            .arg("--origin")
            .arg(origin.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn cmd_unpack(origin: &Path, unpacked: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("unpack")
            .arg("--origin")
            .arg(origin.to_str().unwrap())
            .arg("--unpacked")
            .arg(unpacked.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Push pipeline through the CLI
    async fn cmd_push(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("push")
            .arg("--dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Pull pipeline through the CLI
    async fn cmd_pull(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pull").arg("--dir").arg(dir.to_str().unwrap());
        Ok(cmd)
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
        cmd_init(&origin).await?.assert().success();
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
        cmd_init(origin).await?.assert().success();
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_some());
        // Deinitialize the directory
        cmd_deinit(origin).await?.assert().success();
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
        cmd_init(&input_dir).await?.assert().success();

        // Configure remote endpoint
        cmd_configure_remote("http://127.0.0.1:5001")
            .await?
            .assert()
            .success();

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
        cmd_init(origin).await?.assert().success();
        // Run pack and assert success
        cmd_pack(origin).await?.assert().success();
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
        cmd_init(origin).await?.assert().success();
        // Run pack and assert success
        cmd_pack(origin).await?.assert().success();
        // Create unpacked dir
        let unpacked = &origin.parent().unwrap().join("unpacked");
        create_dir(unpacked).ok();
        // Run unpack and assert success
        cmd_unpack(origin, unpacked).await?.assert().success();
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
        cmd_init(origin).await?.assert().success();
        // Configure remote endpoint
        cmd_configure_remote("http://127.0.0.1:5001")
            .await?
            .assert()
            .success();
        // Run pack locally and assert success
        cmd_pack(origin).await?.assert().success();

        let v1_path = &GlobalConfig::from_disk()?
            .get_bucket(origin)
            .unwrap()
            .content
            .path;
        let v1_moved = &v1_path.parent().unwrap().join("old_content.car");
        file::move_file(v1_path, v1_moved, &file::CopyOptions::new())?;

        // Run push and assert success
        cmd_push(origin).await?.assert().success();
        // Run unpack and assert success
        cmd_pull(origin).await?.assert().success();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(metadata(v1_path)?.len(), metadata(v1_moved)?.len(),);
        // Teardown test
        test_teardown(test_name).await
    }
}
