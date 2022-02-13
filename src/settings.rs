/// Module for managing the settings (command line arguments) of the app

use std::fmt;
use std::str::FromStr;
use std::path::PathBuf;
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

#[derive(PartialEq)]
pub enum GapSearchMode {
    GapSearchFromStart,
    NoGapSearch,
    GapSearchAnywere,
}

impl Default for GapSearchMode {
    fn default() -> Self { Self::GapSearchFromStart }
}

impl fmt::Display for GapSearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>)  -> fmt::Result {
        let text = match self {
            GapSearchMode::GapSearchFromStart => "gap search from start",
            GapSearchMode::NoGapSearch        => "normal search",
            GapSearchMode::GapSearchAnywere   => "gap search anywhere",
        };
        write!(f, "{}", text)
    }
}


#[derive(Default)]
pub struct TereSettings {
    //TODO: options to show non-folders faintly, and skip over them with cursor (in ui settings) -- does this make sense?
    pub folders_only: bool,
    /// If true, show only items matching the search in listing
    pub filter_search: bool,

    pub case_sensitive: CaseSensitiveMode,

    pub autocd_timeout: Option<u64>,

    pub history_file: Option<PathBuf>,

    /// whether to allow matches with gaps in them, and if we have to match from beginning
    pub gap_search_mode: GapSearchMode,
}

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Result<Self, clap::Error> {
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

        if args.is_present("gap-search") {
            ret.gap_search_mode = GapSearchMode::GapSearchFromStart;
        } else if args.is_present("gap-search-anywhere") {
            ret.gap_search_mode = GapSearchMode::GapSearchAnywere;
        } else if args.is_present("no-gap-search") {
            ret.gap_search_mode = GapSearchMode::NoGapSearch;
        }

        ret.autocd_timeout = match args.values_of("autocd-timeout")
            // ok to unwrap because autocd-timeout has a default value which is always present
            .unwrap().last().unwrap()
        {
            "off" => None,
            x => u64::from_str(x).map_err(|_| {
                // We don't want to pass the App all the way here, so create raw error
                // NOTE: We don't call error.format(app) anywhere now, but it doesn't seem to
                // make a difference for this error type.
                clap::Error::raw(
                    clap::ErrorKind::InvalidValue,
                    format!("Invalid value for 'autocd-timeout': '{}'\n", x)
                )
            })?.into()
        };


        if let Some(hist_file) = args.value_of("history-file") {
            ret.history_file = if hist_file.is_empty() {
                None
            } else {
                Some(PathBuf::from(hist_file))
            }
        } else {
            ret.history_file = dirs::cache_dir().map(|path| path
                                                     .join(env!("CARGO_PKG_NAME"))
                                                     .join("history.json"));
        }

        Ok(ret)
    }
}
