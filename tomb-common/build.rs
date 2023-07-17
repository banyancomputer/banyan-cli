fn report_build_profile() {
    println!(
        "cargo:rustc-env=BUILD_PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
}

fn report_enabled_features() {
    let mut enabled_features: Vec<&str> = Vec::new();
    if enabled_features.is_empty() {
        enabled_features.push("none");
    }
    println!(
        "cargo:rustc-env=BUILD_FEATURES={}",
        enabled_features.join(",")
    );
}

fn report_repository_version() {
    let git_describe = std::process::Command::new("git")
        .args(["describe", "--always", "--dirty", "--long", "--tags"])
        .output()
        .unwrap();

    let long_version = String::from_utf8(git_describe.stdout).unwrap();
    println!("cargo:rustc-env=REPO_VERSION={}", long_version);
}

fn main() {
    report_repository_version();
    report_build_profile();
    report_enabled_features();
}
