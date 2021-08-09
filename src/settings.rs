/// Module for managing the settings (command line arguments) of the app

use clap::ArgMatches;

//TODO: config file

//TODO: separate struct for "UI settings" which is accessible by the TereTui struct

#[derive(Default)]
pub struct TereSettings {
    //TODO: options to show non-folders faintly, and skip over them with cursor (in ui settings) -- does this make sense?
    pub folders_only: bool,
    //// if this is true, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool, // TODO
    //case_insensitive: bool //TODO: case insensitive search
    /// If true, show only items matching the search in listing
    pub filter_search: bool,

    pub case_sensitive: bool,
}

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Self {
        let mut ret = Self::default();

        if args.is_present("folders-only") {
            ret.folders_only = true;
        }

        if args.is_present("filter-search") {
            ret.filter_search = true;
        }

        if args.is_present("case-sensitive") {
            ret.case_sensitive = true;
        }

        ret
    }
}
