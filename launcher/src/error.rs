use std::io;
use crate::profile::LaunchProfileError;

#[derive(Debug)]
pub(crate) enum LaunchError {
    ConfigError(LaunchProfileError),
    SteamFileCreationError(io::Error),
    ModDirectoryCreationError(io::Error),
    ModIndexingError(io::Error),
}

