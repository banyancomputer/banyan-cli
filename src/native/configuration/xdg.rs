use std::{env, fs::create_dir_all, path::PathBuf};
const HOME_ERROR: &str = "cant find home directory";
const GLOBAL_CONFIG_FILE_NAME: &str = "config.json";
const DEVICE_API_KEY_FILE_NAME: &str = "device_api_key.pem";
const DEVICE_WRAPPING_KEY_FILE_NAME: &str = "wrapping_key.pem";

/// Grab config path
pub fn xdg_config_home() -> PathBuf {
    // Construct
    let path = PathBuf::from(format!(
        "{}/.config/banyan",
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
        "{}/.local/share/banyan",
        env::var("HOME").expect(HOME_ERROR)
    ));
    // If the directory doesnt exist yet, make it!
    if !path.exists() {
        create_dir_all(&path).expect("failed to create XDG data home");
    }
    // Return
    path
}

/// Grab path to config.json File
pub fn config_path() -> PathBuf {
    xdg_config_home().join(GLOBAL_CONFIG_FILE_NAME)
}

/// Grab path to API Key
pub fn default_api_key_path() -> PathBuf {
    xdg_config_home().join(DEVICE_API_KEY_FILE_NAME)
}

/// Grab path to Wrapping Key
pub fn default_wrapping_key_path() -> PathBuf {
    xdg_config_home().join(DEVICE_WRAPPING_KEY_FILE_NAME)
}
