/// This module contains functionality for checking if the app is being run for the first time, or
/// if it has already been "installed".
use std::path::PathBuf;

use crate::settings::TereSettings;

use crate::error::TereError;

/// Determine whether the app is being run for the first time, and if so, prompt the user to
/// configure their shell. If the app has been run before, or the user responds affirmatively to
/// the prompt, write the `version` file and return Ok, otherwise return an error.
pub fn check_first_run_with_prompt(settings: &TereSettings) -> Result<(), TereError> {
    let hist_file = &settings.history_file;
    let version_file = version_file_path();

    // Check if the version file exists to determine whether we want to show the first run prompt.
    // Additionally, to be backwards compatible and not show the prompt to old users, use a bit of a
    // heuristic: we assume the app has been run before if the history file exists, or the user
    // explicitly requests no history file.
    if version_file.is_none() // chache dir doesn't exist, we assume that the user knows what they're doing
        || version_file.unwrap().try_exists().unwrap_or(false) // version file exists
        || hist_file.is_none() // user passed empty history file
        || PathBuf::from(hist_file.as_ref().unwrap()).try_exists().unwrap_or(false) // history file exists
    {
        //TODO: write version file
        Ok(())
    } else {
        prompt_first_run()
    }
}

fn prompt_first_run() -> Result<(), TereError> {
    todo!()
}

/// Get path for the `version` file. Returns None if the cache folder doesn't exist.
fn version_file_path() -> Option<PathBuf> {
    dirs::cache_dir().map(|path| path.join(env!("CARGO_PKG_NAME")).join("version"))
}

fn write_version_file() -> Result<(), TereError> {
    todo!()
}
