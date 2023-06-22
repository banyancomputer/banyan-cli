use clap::{Parser, Subcommand};
use log::LevelFilter;
use std::path::PathBuf;
use tomb_common as _;

// TODO add support for https://docs.rs/keyring/latest/keyring/
// TODO what's going on with buckets? these are URLs right?

#[derive(Subcommand, Clone, Debug)]
pub(crate) enum ConfigSubCommands {
    /// new content scratch path
    ContentScratchPath {
        #[arg(
            short,
            long,
            help = "content scratch path- should be a disk of decent size where we can use it as a scratch space to build car files en route to filecoin"
        )]
        path: PathBuf,
    },
    /// tomb seturl - Set the ID for this tomb's bucket - MAY BREAK YOUR EVERYTHING!!!
    SetRemote {
        /// Server address
        #[arg(short, long, help = "full server address")]
        address: String,
    },
}

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        input_dir: Option<PathBuf>,

        // /// Maximum size for each chunk, defaults to 1GiB.
        // #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        // chunk_size: u64,
        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    Unpack {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "input directory")]
        input_dir: PathBuf,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,
    },
    Add {
        #[arg(short, long, help = "new file / directory")]
        input_file: PathBuf,
        #[arg(short, long, help = "new file / directory")]
        tomb_path: PathBuf,
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    Remove {
        #[arg(short, long, help = "new file / directory")]
        tomb_path: PathBuf,
        #[arg(short, long, help = "wnfs path")]
        wnfs_path: PathBuf,
    },
    /// tomb pull - Update local from the bucket- determined by CWD
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
    ///- Initialize Tomb - Abort if the `~/.tomb` path already exists
    //- Create a new directory at `~/.tomb`
    //- Create a new config file at `~/.tomb/config`:
    //    - `metadata_path: ~/.tomb/metadata`
    //    - `content_path: ~/.tomb/content`
    //    - `bucket_path: ~./tomb/buckets`
    //    - `tombolo_path: ~/.tomb/olo`
    //    - `index_path: ~/.tomb/index`
    /// tomb init - create a new .tomb file and populate it.
    Init {
        dir: Option<PathBuf>,
    },
    Deinit {
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
        #[clap(subcommand)]
        subcommand: ConfigSubCommands,
    },
    Daemon,
}

#[derive(Clone, Debug, clap::ValueEnum)]
pub(crate) enum MyVerbosity {
    Quiet,
    Normal,
    Verbose,
    VeryVerbose,
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

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub(crate) command: Commands,

    /// Verbosity level.
    #[arg(short, long, help = "verbosity level", default_value = "normal")]
    pub(crate) verbose: MyVerbosity,
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use fs_extra::file;
    use serial_test::serial;
    use std::{fs::metadata, path::Path, process::Command};
    use tomb::utils::tests::{test_setup, test_teardown};
    use tomb_common::types::config::globalconfig::GlobalConfig;

    async fn init(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("init").arg(dir);
        Ok(cmd)
    }

    async fn deinit(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("deinit").arg(dir);
        Ok(cmd)
    }

    async fn configure_remote(address: &str) -> Result<Command> {
        // configure set-remote --url http://127.0.0.1 --port 5001
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("configure")
            .arg("set-remote")
            .arg("--address")
            .arg(address);
        Ok(cmd)
    }

    // Run the Pack pipeline through the CLI
    async fn pack(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn unpack(input_dir: &Path, output_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("unpack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Push pipeline through the CLI
    async fn push(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("push")
            .arg("--dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Pull pipeline through the CLI
    async fn pull(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pull").arg("--dir").arg(dir.to_str().unwrap());
        Ok(cmd)
    }

    #[tokio::test]
    #[serial]
    async fn cli_init() -> Result<()> {
        let test_name = "cli_init";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Initialization worked
        init(&origin).await?.assert().success();
        // Assert the bucket exists now
        let bucket = GlobalConfig::from_disk()?.get_bucket(origin);
        assert!(bucket.is_some());
        // Assert that there is still no key, because we've not packed
        assert!(bucket.unwrap().get_key("root").is_err());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn cli_init_deinit() -> Result<()> {
        let test_name = "cli_init_deinit";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Assert no bucket exists yet
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Initialization worked
        init(origin).await?.assert().success();
        // Assert the bucket exists now
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_some());
        // Deinitialize the directory
        deinit(origin).await?.assert().success();
        // Assert the bucket is gone again
        assert!(GlobalConfig::from_disk()?.get_bucket(origin).is_none());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn cli_configure_remote() -> Result<()> {
        let test_name = "cli_configure_remote";
        // Setup test
        let input_dir = &test_setup(test_name).await?;

        // Initialize
        init(&input_dir).await?.assert().success();

        // Configure remote endpoint
        configure_remote("http://127.0.0.1:5001")
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
    async fn cli_pack_local() -> Result<()> {
        let test_name = "cli_pack_local";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        init(origin).await?.assert().success();
        // Run pack and assert success
        pack(origin).await?.assert().success();
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn cli_unpack_local() -> Result<()> {
        let test_name = "cli_unpack_local";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        init(origin).await?.assert().success();
        // Run pack and assert success
        pack(origin).await?.assert().success();
        // Run unpack and assert success
        unpack(origin, origin).await?.assert().success();
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    #[ignore]
    async fn cli_push_pull() -> Result<()> {
        let test_name = "cli_push_pull";
        // Setup test
        let origin = &test_setup(test_name).await?;
        // Initialize tomb
        init(origin).await?.assert().success();
        // Configure remote endpoint
        configure_remote("http://127.0.0.1:5001")
            .await?
            .assert()
            .success();
        // Run pack locally and assert success
        pack(origin).await?.assert().success();

        let v1_path = &GlobalConfig::from_disk()?
            .get_bucket(origin)
            .unwrap()
            .content
            .path;
        let v1_moved = &v1_path.parent().unwrap().join("old_content.car");
        file::move_file(v1_path, v1_moved, &file::CopyOptions::new())?;

        // Run push and assert success
        push(origin).await?.assert().success();
        // Run unpack and assert success
        pull(&origin).await?.assert().success();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(metadata(v1_path)?.len(), metadata(v1_moved)?.len(),);
        // Teardown test
        test_teardown(test_name).await
    }
}
