/// Module for managing the settings (command line arguments) of the app
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;
use clap::ArgMatches;
use crossterm::event::KeyEvent;
use crokey::key;

use crate::ui::Action;

//TODO: config file?

pub enum CaseSensitiveMode {
    IgnoreCase,
    CaseSensitive,
    SmartCase,
}

impl Default for CaseSensitiveMode {
    fn default() -> Self {
        Self::SmartCase
    }
}

impl fmt::Display for CaseSensitiveMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    fn default() -> Self {
        Self::GapSearchFromStart
    }
}

impl fmt::Display for GapSearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
    /// If true, show only folders, not files in the listing
    pub folders_only: bool,
    /// If true, show only items matching the search in listing
    pub filter_search: bool,

    pub case_sensitive: CaseSensitiveMode,

    pub autocd_timeout: Option<u64>,

    pub history_file: Option<PathBuf>,

    /// whether to allow matches with gaps in them, and if we have to match from beginning
    pub gap_search_mode: GapSearchMode,

    pub mouse_enabled: bool,

    pub keymap: HashMap<KeyEvent, Action>,
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

        ret.autocd_timeout = match args
            .values_of("autocd-timeout")
            // ok to unwrap because autocd-timeout has a default value which is always present
            .unwrap()
            .last()
            .unwrap()
        {
            "off" => None,
            x => u64::from_str(x)
                .map_err(|_| {
                    // We don't want to pass the App all the way here, so create raw error
                    // NOTE: We don't call error.format(app) anywhere now, but it doesn't seem to
                    // make a difference for this error type.
                    clap::Error::raw(
                        clap::ErrorKind::InvalidValue,
                        format!("Invalid value for 'autocd-timeout': '{}'\n", x),
                    )
                })?
                .into(),
        };

        if let Some(hist_file) = args.value_of("history-file") {
            ret.history_file = if hist_file.is_empty() {
                None
            } else {
                Some(PathBuf::from(hist_file))
            }
        } else {
            ret.history_file = dirs::cache_dir()
                .map(|path| path.join(env!("CARGO_PKG_NAME")).join("history.json"));
        }

        // ok to unwrap, because mouse has the default value of 'off'
        if args.values_of("mouse").unwrap().last().unwrap() == "on" {
            ret.mouse_enabled = true;
        }

        //ret.keymap = HashMap::from_iter(DEFAULT_KEYMAP.iter());
        ret.keymap = DEFAULT_KEYMAP.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        Ok(ret)
    }
}

// NOTE: can't create a const hashmap (without an extra dependency like phf), so just using a slice
// of tuples.
const DEFAULT_KEYMAP: &[(KeyEvent, Action)] = &[

    (key!(enter),    Action::ChangeDir),
    (key!(right),    Action::ChangeDir),
    (key!(alt-down), Action::ChangeDir),
    (key!(alt-l),    Action::ChangeDir),
    //(key!(space), Action::ChangeDir & NotSearching), //TODO: figure out quantifiers...

    (key!(left),   Action::ChangeDirParent),
    (key!(alt-up), Action::ChangeDirParent),
    (key!(alt-h),  Action::ChangeDirParent),
    //(key!('-'),    Action::ChangeDirParent & NotSearching), // TODO: quantifier
    //(key!(backspace),    Action::ChangeDirParent & NotSearching), // TODO: quantifier

    (key!('~'), Action::ChangeDirHome),
    (key!(ctrl-home), Action::ChangeDirHome),
    (key!(ctrl-alt-h), Action::ChangeDirHome),

    (key!('/'), Action::ChangeDirRoot),
    (key!(alt-r), Action::ChangeDirRoot),

    (key!(up),    Action::CursorUp),
    (key!(alt-k), Action::CursorUp),

    (key!(down),  Action::CursorDown),
    (key!(alt-j), Action::CursorDown),

    (key!(pageup),  Action::CursorUpPage),
    (key!(alt-u),   Action::CursorUpPage),
    (key!(ctrl-u),  Action::CursorUpPage),

    (key!(pagedown), Action::CursorDownPage),
    (key!(alt-d),    Action::CursorDownPage),
    (key!(ctrl-d),   Action::CursorDownPage),

    (key!(home),  Action::CursorFirst),
    (key!(alt-g), Action::CursorFirst),
    (key!(end),   Action::CursorLast),
    (key!(alt-shift-g), Action::CursorLast),

    (key!(alt-c), Action::ChangeCaseSensitiveMode),
    (key!(ctrl-f), Action::ChangeGapSearchMode),

    (key!(ctrl-r), Action::RefreshListing),

    (key!('?'), Action::Help),

    (key!(esc), Action::ClearSearchOrExit), //TODO: use quantifier instead...
    (key!(alt-q), Action::Exit),
    (key!(ctrl-c), Action::ExitWithoutCd),

];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_default_keymap_keys_unique() {
        let mut key_counts: HashMap<KeyEvent, usize> = HashMap::new();

        DEFAULT_KEYMAP
            .iter()
            .for_each(|(k, v)| *key_counts.entry(*k).or_default() += 1);

        for (k, v) in key_counts {
            assert_eq!(v, 1, "found {} entries for key {:?}", v, k);
        }
    }
}
