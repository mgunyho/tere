/// Module for managing the settings (command line arguments) of the app

use clap::ArgMatches;

//TODO: config file

#[derive(Default)]
pub struct TereSettings {
    pub folders_only: bool,
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
