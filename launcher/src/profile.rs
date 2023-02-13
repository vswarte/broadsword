use std::io;
use serde::Deserialize;

#[derive(Debug)]
pub(crate) enum LaunchProfileError {
    FileReadError(io::Error),
    ConfigParseError,
}

pub(crate) fn read_launch_profile(path: &str) -> Result<LaunchProfile, LaunchProfileError> {
    let launch_profile_contents = std::fs::read_to_string(path)
        .map_err(|e| LaunchProfileError::FileReadError(e))?;

    let result = toml::from_str(launch_profile_contents.as_str())
        .map_err(|_| LaunchProfileError::ConfigParseError)?;

    Ok(result)
}

#[derive(Debug, Deserialize)]
pub(crate) struct LaunchProfile {
    pub executable: String,
    pub steam_app_id: Option<String>,

    #[serde(default = "default_steam_app_id_file")]
    pub steam_app_id_file: String,

    #[serde(default = "default_mod_folder")]
    pub mod_folder: String,
}


fn default_steam_app_id_file() -> String {
    "./steam_appid.txt".to_string()
}

fn default_mod_folder() -> String {
    "./mods".to_string()
}
