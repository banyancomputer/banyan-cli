#[cfg(test)]
mod test {
    use assert_cmd::prelude::*;
    use predicates::prelude::*;
    use tomb::utils::{tests::{test_setup, test_teardown}}; // Used for writing assertions
    use std::process::Command; // Run programs
    use anyhow::Result;

    #[tokio::test]
    async fn cli_pack() -> Result<()> {
        // Setup test
        let (input_dir, output_dir) = test_setup("cli_pack").await?;

        let mut cmd = Command::cargo_bin("tomb-cli")?;
        cmd.arg("pack")
            .arg("--input-dir").arg(input_dir.to_str().unwrap())
            .arg("--output-dir").arg(output_dir.to_str().unwrap());

        // Run and assure success
        cmd.assert().success();

        test_teardown("cli_pack").await
    }
}