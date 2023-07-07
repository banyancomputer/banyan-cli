use std::{env, fs::create_dir, path::PathBuf};

const HOME_ERROR: &str = "cant find home directory";

/// Grab config path from env variables + XDG spec
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.config/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    create_dir(&path).ok();
    // Return
    path
}

pub fn xdg_data_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.local/share/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    create_dir(&path).ok();
    // Return
    path
}
