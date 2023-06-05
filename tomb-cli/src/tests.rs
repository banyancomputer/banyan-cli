#[cfg(test)]
mod test {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use fs_extra::dir::CopyOptions;
    use predicates::prelude::*;
    use serial_test::serial;
    use std::{
        fs::{self, create_dir_all},
        path::Path,
        process::Command,
    };
    use tomb::utils::{
        serialize::load_manifest,
        tests::{compute_directory_size, test_setup, test_teardown},
    };
    use tomb_common::types::pipeline::Manifest;

    async fn init(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("init").arg("--dir").arg(dir);
        Ok(cmd)
    }

    async fn configure_remote(dir: &Path, url: &str, port: u16) -> Result<Command> {
        // configure set-remote --url http://127.0.0.1 --port 5001
        let mut cmd = Command::cargo_bin("tomb-cli")?;
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
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Pack pipeline through the CLI
    async fn pack_remote(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn unpack(input_dir: &Path, output_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("unpack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn push(input_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("push")
            .arg("--dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn pull(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pull").arg("--dir").arg(dir.to_str().unwrap());
        Ok(cmd)
    }

    #[tokio::test]
    async fn cli_init() -> Result<()> {
        // Setup test
        let (input_dir, _) = &test_setup("cli_init").await?;
        // Initialization worked
        init(&input_dir).await?.assert().success();
        // Load the modified Manifest
        let manifest = load_manifest(&input_dir.join(".tomb"))?;
        // Expect that the default Manifest was successfully encoded
        assert_eq!(manifest, Manifest::default());
        // Teardown test
        test_teardown("cli_init").await
    }

    #[tokio::test]
    async fn cli_configure_remote() -> Result<()> {
        // Setup test
        let (input_dir, _) = &test_setup("cli_remote").await?;
        // Initialization worked
        init(&input_dir).await?.assert().success();

        // Configure remote endpoint
        configure_remote(&input_dir, "http://127.0.0.1", 5001)
            .await?
            .assert()
            .success();

        // Load the modified Manifest
        let manifest = load_manifest(&input_dir.join(".tomb"))?;
        // Expect that the remote endpoint was successfully updated
        assert_eq!(manifest.content_remote.addr, "http://127.0.0.1:5001");
        // Teardown test
        test_teardown("cli_remote").await
    }

    #[tokio::test]
    async fn cli_pack_local() -> Result<()> {
        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_pack_local").await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Run pack and assert success
        pack_local(input_dir, output_dir).await?.assert().success();
        // Teardown test
        test_teardown("cli_pack_local").await
    }

    #[tokio::test]
    #[serial]
    async fn cli_pack_remote() -> Result<()> {
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();
        // Setup test
        let (input_dir, _) = &test_setup("cli_pack_remote").await?;
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
        test_teardown("cli_pack_remote").await
    }

    #[tokio::test]
    async fn cli_unpack() -> Result<()> {
        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_unpack").await?;
        // Initialize tomb
        init(&input_dir).await?.assert().success();
        // Run pack and assert success
        pack_local(input_dir, output_dir).await?.assert().success();
        // Run unpack and assert success
        unpack(output_dir, input_dir).await?.assert().success();
        // Teardown test
        test_teardown("cli_unpack").await
    }

    #[tokio::test]
    #[serial]
    async fn cli_push_pull() -> Result<()> {
        // Start the IPFS daemon
        // let mut ipfs = start_daemon();

        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_push_pull").await?;
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
        // push(output_dir).await?.assert().success();
        // Create a directory in which to reconstruct
        // let rebuild_dir = output_dir.parent().unwrap().join("rebuild");
        // create_dir_all(&rebuild_dir)?;
        // // Copy the metadata into the new directory, but no content
        // fs_extra::copy_items(&[output_dir.join(".tomb")], &rebuild_dir, &CopyOptions::new())?;
        // // Run unpack and assert success
        // pull(&rebuild_dir).await?.assert().success();
        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        // assert_eq!(compute_directory_size(&output_dir.join("content"))?, compute_directory_size(&rebuild_dir.join("content"))?);

        // Kill the daemon
        // ipfs.kill()?;

        // Teardown test
        test_teardown("cli_push_pull").await
    }
}
