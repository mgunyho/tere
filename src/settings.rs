/// Module for managing the settings (command line arguments) of the app

use std::fmt;
use std::str::FromStr;
use clap::ArgMatches;

//TODO: config file

//TODO: separate struct for "UI settings" which is accessible by the TereTui struct

pub enum CaseSensitiveMode {
    IgnoreCase,
    CaseSensitive,
    SmartCase,
}

impl Default for CaseSensitiveMode {
    fn default() -> Self { Self::SmartCase }
}

impl fmt::Display for CaseSensitiveMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>)  -> fmt::Result {
        let text = match self {
            CaseSensitiveMode::IgnoreCase    => "ignore case",
            CaseSensitiveMode::CaseSensitive => "case sensitive",
            CaseSensitiveMode::SmartCase     => "smart case",
        };
        write!(f, "{}", text)
    }
}


#[derive(Default)]
pub struct TereSettings {
    //TODO: options to show non-folders faintly, and skip over them with cursor (in ui settings) -- does this make sense?
    pub folders_only: bool,
    //// if this is true, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool, // TODO
    //case_insensitive: bool //TODO: case insensitive search
    /// If true, show only items matching the search in listing
    pub filter_search: bool,

    pub case_sensitive: CaseSensitiveMode,

    pub autocd_timeout: Option<u64>,
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
            ret.case_sensitive = CaseSensitiveMode::CaseSensitive;
        } else if args.is_present("ignore-case") {
            ret.case_sensitive = CaseSensitiveMode::IgnoreCase;
        } else if args.is_present("smart-case") {
            ret.case_sensitive = CaseSensitiveMode::SmartCase;
        }

        ret.autocd_timeout = match args.values_of("autocd-timeout")
            // ok to unwrap because autocd-timeout has a default value
            .unwrap().last().unwrap()
        {
            "off" => None,
            x => Some(
                u64::from_str(x)
                .unwrap_or_else(|_| clap::Error::with_description(
                        //TODO: color? (should be consistent with other errors) - could use `Arg::Validator`
                        &format!("Invalid value for 'autocd-timeout': '{}'", x),
                        clap::ErrorKind::InvalidValue
                        // TODO: return error instead of exiting immediately, for better cleanup
                        ).exit()
                    )
            ),
        };

        ret
    }
}
