/// Module for managing the settings (command line arguments) of the app

use clap::{ArgMatches, arg_enum, value_t};

//TODO: config file

//TODO: separate struct for "UI settings" which is accessible by the TereTui struct

arg_enum! {
    /// Enum corresponding to the 'non-folders' CLI arg
    pub enum NonFoldersOption {
        True,
        False,
        Skip,
    }
}

impl Default for NonFoldersOption {
    fn default() -> Self { Self::True }
}

#[derive(Default)]
pub struct TereSettings {
    //TODO: options to show non-folders faintly, and skip over them with cursor (in ui settings) -- does this make sense?
    pub show_nonfolders: NonFoldersOption,
    //// if this is true, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool, // TODO
    //case_insensitive: bool //TODO: case insensitive search
}

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Self {
        let mut ret = Self::default();

        ret.show_nonfolders = value_t!(args, "non-folders", NonFoldersOption)
            .unwrap_or_else(|e| e.exit());

        ret
    }
}
