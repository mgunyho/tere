/// Module for managing the settings (command line arguments) of the app

use clap::ArgMatches;

//TODO: config file

#[derive(Default)]
pub struct TereSettings {
    pub folders_only: bool,
    //// if this is true, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool, // TODO
    //case_insensitive: bool //TODO: case insensitive search
}

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Self {
        let mut ret = Self::default();

        if args.is_present("folders-only") {
            ret.folders_only = true;
        }

        ret
    }
}
