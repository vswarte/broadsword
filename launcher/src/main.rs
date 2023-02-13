use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::error::LaunchError;
use dll_syringe::{process::OwnedProcess, Syringe};

const DEFAULT_LAUNCH_PROFILE_PATH: &str = "./broadsword_launcher.toml";

mod error;
mod steam;
mod profile;

fn main() {
    // Read profile path from env vars, use default is none is specified
    let profile_path = env::var("BROADSWORD_PROFILE")
        .map(|x| x.as_str())
        .unwrap_or(DEFAULT_LAUNCH_PROFILE_PATH);

    // Read and parse the specified launch profile file
    let launch_profile = profile::read_launch_profile(profile_path)
        .map_err(|e| LaunchError::ConfigError(e))
        .expect("Could not read launch profile");

    // Create the mod directory if it's not there already
    initialize_mod_directory(launch_profile.mod_folder.as_str())
        .expect("Could not initialize mod directory");

    // Create the steam app ID file if an app ID is specified and it the file doesn't exist yet
    if launch_profile.steam_app_id.is_some() {
        steam::create_steam_app_id_file(&launch_profile);
    }

    // Gather the loadable DLLs from the mod folder
    let loadable_dlls = get_loadable_dlls(launch_profile.mod_folder.as_str())
        .expect("Could not get loadable mods");

    // Launch the executable specified in the launch profile
    Command::new(launch_profile.executable.as_str())
        .spawn()
        .expect("Failed to launch game");

    // Find the target process and create an injector
    let target_process = OwnedProcess::find_first_by_name(launch_profile.executable.as_str())
        .expect("Could not find game's process");
    let injector = Syringe::for_process(target_process);

    for dll in loadable_dlls {
        injector
            .inject(&dll)
            .expect(&format!("Could not inject DLL {}", &dll.to_string_lossy()));
    }
}

fn initialize_mod_directory(mod_directory: &str) -> Result<(), LaunchError> {
    if !Path::new(mod_directory).exists() {
        fs::create_dir(mod_directory)
            .map_err(|e| LaunchError::SteamFileCreationError(e))?;
    }

    Ok(())
}

fn get_loadable_dlls(mod_directory: &str) -> Result<Vec<PathBuf>, LaunchError> {
    let dir_entries = fs::read_dir(mod_directory)
        .map_err(|e| LaunchError::ModIndexingError(e))?;

    let entries = dir_entries
        .filter_map(|x| {
            x.ok().and_then(|y| {
                let extension = y.path().extension();
                if extension.is_some() && extension.unwrap() == "dll" {
                    Some(y.path())
                } else {
                    None
                }
            })
        })
        .collect();

    Ok(entries)
}
