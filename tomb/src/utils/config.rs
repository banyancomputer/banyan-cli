use std::{env, fs::create_dir_all, path::PathBuf};

const HOME_ERROR: &str = "cant find home directory";

/// Grab config path
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.config/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG config home");
    }
    // Return
    path
}

/// Grab data path
pub fn xdg_data_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.local/share/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG data home");
    }
    // Return
    path
}
