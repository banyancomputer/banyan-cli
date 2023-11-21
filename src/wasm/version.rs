pub fn version() -> String {
    format!(
        "build-profile={}, repo-version={}",
        env!("BUILD_PROFILE"),
        env!("REPO_VERSION")
    )
}
