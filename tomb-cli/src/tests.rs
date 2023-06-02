#[cfg(test)]
mod test {
    use anyhow::Result;
    use assert_cmd::prelude::*;
    use fs_extra::dir::CopyOptions;
    use predicates::prelude::*;
    use tomb_common::utils::{tomb_config, get_remote};
    use std::{
        fs::{self, create_dir_all},
        path::Path,
        process::Command,
    };
    use tomb::utils::tests::{compute_directory_size, start_daemon, test_setup, test_teardown};

    // Run the Pack pipeline through the CLI
    async fn pack(input_dir: &Path, output_dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pack")
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap())
            .arg("--output-dir")
            .arg(output_dir.to_str().unwrap());
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
            .arg("--input-dir")
            .arg(input_dir.to_str().unwrap());
        Ok(cmd)
    }

    // Run the Unpack pipeline through the CLI
    async fn pull(dir: &Path) -> Result<Command> {
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pull").arg("--dir").arg(dir.to_str().unwrap());
        Ok(cmd)
    }

    async fn configure_remote(url: String, port: u16) -> Result<Command> {
        // configure set-remote --url 127.0.0.1 --port 5001
        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("configure")
            .arg("set-remote")
            .arg("--url")
            .arg(url)
            .arg("--port")
            .arg(format!("{}", port));
        Ok(cmd)
    }

    #[tokio::test]
    async fn cli_pack() -> Result<()> {
        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_pack").await?;
        // Run pack and assert success
        pack(input_dir, output_dir).await?.assert().success();
        // Teardown test
        test_teardown("cli_pack").await
    }

    #[tokio::test]
    async fn cli_unpack() -> Result<()> {
        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_unpack").await?;
        // Run pack and assert success
        pack(input_dir, output_dir).await?.assert().success();
        // Run unpack and assert success
        unpack(output_dir, input_dir).await?.assert().success();
        // Teardown test
        test_teardown("cli_unpack").await
    }

    #[tokio::test]
    async fn cli_configure_remote() -> Result<()> {
        // Remove existing configuration
        fs::remove_file(tomb_config()?.join("remote"))?;

        // Expect that we are not able to get the remote config
        get_remote().expect_err("msg");

        let url = String::from("127.0.0.1");
        let port: u16 = 5001;

        // Configure remote endpoint
        configure_remote(url.clone(), port)
            .await?
            .assert()
            .success();

        let (new_url, new_port) = get_remote()?;

        assert_eq!(url, new_url);
        assert_eq!(port, new_port);

        Ok(())
    }

    #[tokio::test]
    async fn cli_push_pull() -> Result<()> {
        // Start the IPFS daemon
        let mut ipfs = start_daemon();

        // Setup test
        let (input_dir, output_dir) = &test_setup("cli_push_pull").await?;
        // Run pack and assert success
        pack(input_dir, output_dir).await?.assert().success();
        // Configure remote endpoint if not already done
        configure_remote(String::from("127.0.0.1"), 5001)
            .await?
            .assert()
            .success();
        // Run unpack and assert success
        push(output_dir).await?.assert().success();

        // Create a directory in which to reconstruct
        let rebuild_dir = output_dir.parent().unwrap().join("rebuild");
        create_dir_all(&rebuild_dir)?;

        // output_dir.
        fs_extra::copy_items(&[output_dir], &rebuild_dir, &CopyOptions::new())?;
        let rebuild_dir = rebuild_dir.join("output");
        // Remove data. TODO (organizedgrime) only remove SOME data to ensure partial reconstruction capacity.
        fs::remove_file(rebuild_dir.join("content").join("1.car"))?;

        // Run unpack and assert success
        pull(&rebuild_dir).await?.assert().success();

        // Compute size of original and reconstructed content
        let d1 = compute_directory_size(&output_dir.join("content")).unwrap();
        let d2 = compute_directory_size(&rebuild_dir.join("content")).unwrap();

        // Assert that, despite reordering of CIDs, content CAR is the exact same size
        assert_eq!(d1, d2);

        // Kill the daemon
        ipfs.kill()?;

        // Teardown test
        test_teardown("cli_push_pull").await
    }
}
