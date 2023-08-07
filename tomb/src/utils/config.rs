use std::{fs::create_dir_all, path::PathBuf};

#[cfg(not(debug_assertions))]
const HOME_ERROR: &str = "cant find home directory";

const CREATE_ERROR: &str = "failed to create directory";

/// Grab config path
#[cfg(not(debug_assertions))]
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.config/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect(CREATE_ERROR);
    }
    // Return
    path
}

/// Grab fake config path
#[cfg(debug_assertions)]
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from("test/.config/tomb");
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect(CREATE_ERROR);
    }
    // Return
    path
}

/// Grab data path
#[cfg(not(debug_assertions))]
pub fn xdg_data_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.local/share/tomb",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect(CREATE_ERROR);
    }
    // Return
    path
}

/// Grab fake data path
#[cfg(debug_assertions)]
pub fn xdg_data_home() -> PathBuf {
    // Construct
    let path = PathBuf::from("test/.local/share/tomb");
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect(CREATE_ERROR);
    }
    // Return
    path
}
