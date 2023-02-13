use std::fs;
use std::path;
use std::io::Write;
use crate::error::LaunchError;
use crate::profile::LaunchProfile;

pub(crate) fn create_steam_app_id_file(profile: &LaunchProfile) -> Result<(), LaunchError> {
    let file_path = &profile.steam_app_id_file;

    if !path::Path::new(file_path).exists() {
        let mut file = fs::File::create(file_path)
            .map_err(|e| LaunchError::SteamFileCreationError(e))?;

        file.write_all(profile.steam_app_id.unwrap().as_bytes())
            .map_err(|e| LaunchError::SteamFileCreationError(e))?;
    }

    Ok(())
}
