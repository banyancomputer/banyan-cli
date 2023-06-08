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
        /// Input directory
        #[arg(short, long, help = "directory")]
        dir: Option<PathBuf>,
        /// Server address
        #[arg(short, long, help = "remote IPv4 address")]
        url: String,
        /// Server port
        #[arg(short, long, help = "remote address port")]
        port: u16,
    },
}

/// Defines the types of commands that can be executed from the CLI.
#[derive(Debug, Subcommand, Clone)]
pub(crate) enum Commands {
    Pack {
        /// Root of the directory tree to pack.
        #[arg(short, long, help = "input directories and files")]
        input_dir: PathBuf,

        /// Directory that either does not exist or is empty; this is where packed data will go.
        #[arg(short, long, help = "output directory")]
        output_dir: Option<PathBuf>,

        /// Maximum size for each chunk, defaults to 1GiB.
        #[arg(short, long, help = "target chunk size", default_value = "1073741824")]
        chunk_size: u64,

        /// Whether to follow symbolic links when processing the input directory.
        #[arg(short, long, help = "follow symbolic links")]
        follow_links: bool,
        // TODO add support for GroupConfig::path_patterns/name_patterns
    },
    Unpack {
        /// Input directory in which packed files are stored.
        #[arg(short, long, help = "input directory")]
        input_dir: Option<PathBuf>,

        /// Output directory in which reinflated files will be unpacked.
        #[arg(short, long, help = "output directory")]
        output_dir: PathBuf,
    },
    Add {
        #[arg(short, long, help = "local")]
        local: bool,
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
        /// Input directory
        #[arg(short, long, help = "directory")]
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
    use fs_extra::dir::CopyOptions;
    use serial_test::serial;
    use std::{fs::create_dir_all, path::Path, process::Command};
    use tomb::utils::{
        disk::manifest_from_disk,
        tests::{compute_directory_size, test_setup, test_teardown},
    };
    use tomb_common::types::pipeline::Manifest;

    async fn init(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("init").arg("--dir").arg(dir);
        Ok(cmd)
    }

    async fn configure_remote(dir: &Path, url: &str, port: u16) -> Result<Command> {
        // configure set-remote --url http://127.0.0.1 --port 5001
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("configure")
            .arg("set-remote")
            .arg("--dir")
            .arg(dir)
            .arg("--url")
            .arg(url)
            .arg("--port")
            .arg(format!("{}", port));
        Ok(cmd)
    }

    // Run the Pack pipeline through the CLI
    async fn pack_local(input_dir: &Path, output_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Pack pipeline through the CLI
    async fn pack_remote(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn unpack_local(input_dir: &Path, output_dir: &Path) -> Result<Command> {
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
    async fn cli_init() -> Result<()> {
        let test_name = "cli_init";
        // Setup test
        let (input_dir, _) = &test_setup(test_name).await?;
        // Initialization worked
        init(&input_dir).await?.assert().success();
        // Load the modified Manifest
        let manifest = manifest_from_disk(&input_dir.join(".tomb"))?;
        // Expect that the default Manifest was successfully encoded
        assert_eq!(manifest, Manifest::default());
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn cli_configure_remote() -> Result<()> {
        let test_name = "cli_configure_remote";
        // Setup test
        let (input_dir, _) = &test_setup(test_name).await?;
        // Initialization worked
        init(&input_dir).await?.assert().success();

        // Configure remote endpoint
        configure_remote(&input_dir, "http://127.0.0.1", 5001)
            .await?
            .assert()
            .success();

        // Load the modified Manifest
        let manifest = manifest_from_disk(&input_dir.join(".tomb"))?;
        // Expect that the remote endpoint was successfully updated
        assert_eq!(manifest.cold_remote.addr, "http://127.0.0.1:5001");
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn cli_pack_local() -> Result<()> {
        let test_name = "cli_pack_local";
        // Setup test
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Run pack and assert success
        pack_local(input_dir, output_dir).await?.assert().success();
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn cli_pack_remote() -> Result<()> {
        let test_name = "cli_pack_remote";
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();
        // Setup test
        let (input_dir, _) = &test_setup(test_name).await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Configure remote endpoint
        configure_remote(&input_dir, "http://127.0.0.1", 5001)
            .await?
            .assert()
            .success();
        // Run pack and assert success
        pack_remote(input_dir).await?.assert().success();
        // Kill the daemon
        // ipfs.kill()?;
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    async fn cli_unpack_local() -> Result<()> {
        let test_name = "cli_unpack_local";
        // Setup test
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Run pack and assert success
        pack_local(input_dir, output_dir).await?.assert().success();
        // Run unpack and assert success
        unpack_local(output_dir, input_dir)
            .await?
            .assert()
            .success();
        // Teardown test
        test_teardown(test_name).await
    }

    #[tokio::test]
    #[serial]
    async fn cli_push_pull() -> Result<()> {
        let test_name = "cli_push_pull";
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();
        // Setup test
        let (input_dir, output_dir) = &test_setup(test_name).await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Configure remote endpoint
        configure_remote(&input_dir, "http://127.0.0.1", 5001)
            .await?
            .assert()
            .success();
        // Run pack locally and assert success
        pack_local(input_dir, output_dir).await?.assert().success();
        // Run push and assert success
        push(output_dir).await?.assert().success();
        // Create a directory in which to reconstruct
        let rebuild_dir = output_dir.parent().unwrap().join("rebuild");
        create_dir_all(&rebuild_dir)?;
        // Copy the metadata into the new directory, but no content
        fs_extra::copy_items(
            &[output_dir.join(".tomb")],
            &rebuild_dir,
            &CopyOptions::new(),
        )?;
        // Run unpack and assert success
        pull(&rebuild_dir).await?.assert().success();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(
            compute_directory_size(&output_dir.join("content"))?,
            compute_directory_size(&rebuild_dir.join("content"))?
        );
        // Kill the daemon
        // ipfs.kill()?;
        // Teardown test
        test_teardown(test_name).await
    }
}
