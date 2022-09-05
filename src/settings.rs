/// Module for managing the settings (command line arguments) of the app
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;
use clap::{ArgMatches, Error as ClapError, error::ErrorKind as ClapErrorKind};
use crossterm::event::KeyEvent;
use crokey::key;

use crate::ui::{Action, ActionContext};

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

    pub keymap: HashMap<(KeyEvent, ActionContext), Action>,
}

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Result<Self, ClapError> {
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
                    ClapError::raw(
                        ClapErrorKind::InvalidValue,
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

        ret.keymap = DEFAULT_KEYMAP.iter()
            .map(|(k, c, a)| ((k.clone(), c.clone()), a.clone()))
            .collect();

        Ok(ret)
    }
}

// NOTE: can't create a const hashmap (without an extra dependency like phf), so just using a slice
// of tuples.
const DEFAULT_KEYMAP: &[(KeyEvent, ActionContext, Action)] = &[

    (key!(enter),    ActionContext::None, Action::ChangeDir),
    (key!(right),    ActionContext::None, Action::ChangeDir),
    (key!(alt-down), ActionContext::None, Action::ChangeDir),
    (key!(alt-l),    ActionContext::None, Action::ChangeDir),
    (key!(space), ActionContext::NotSearching, Action::ChangeDir),

    (key!(left),   ActionContext::None, Action::ChangeDirParent),
    (key!(alt-up), ActionContext::None, Action::ChangeDirParent),
    (key!(alt-h),  ActionContext::None, Action::ChangeDirParent),
    (key!('-'),       ActionContext::NotSearching, Action::ChangeDirParent),
    (key!(backspace), ActionContext::NotSearching, Action::ChangeDirParent),

    (key!('~'),        ActionContext::None, Action::ChangeDirHome),
    (key!(ctrl-home),  ActionContext::None, Action::ChangeDirHome),
    (key!(ctrl-alt-h), ActionContext::None, Action::ChangeDirHome),

    (key!('/'),        ActionContext::None, Action::ChangeDirRoot),
    (key!(alt-r),      ActionContext::None, Action::ChangeDirRoot),

    (key!(alt-enter),  ActionContext::None, Action::ChangeDirAndExit),
    (key!(ctrl-space), ActionContext::None, Action::ChangeDirAndExit),

    (key!(up),    ActionContext::None, Action::CursorUp),
    (key!(alt-k), ActionContext::None, Action::CursorUp),

    (key!(down),  ActionContext::None, Action::CursorDown),
    (key!(alt-j), ActionContext::None, Action::CursorDown),

    (key!(pageup),  ActionContext::None, Action::CursorUpPage),
    (key!(alt-u),   ActionContext::None, Action::CursorUpPage),
    (key!(ctrl-u),  ActionContext::None, Action::CursorUpPage),

    (key!(pagedown), ActionContext::None, Action::CursorDownPage),
    (key!(alt-d),    ActionContext::None, Action::CursorDownPage),
    (key!(ctrl-d),   ActionContext::None, Action::CursorDownPage),

    (key!(home),        ActionContext::None, Action::CursorFirst),
    (key!(alt-g),       ActionContext::None, Action::CursorFirst), // like vim 'gg'
    (key!(end),         ActionContext::None, Action::CursorLast),
    (key!(alt-shift-g), ActionContext::None, Action::CursorLast), // like vim 'G'

    (key!(backspace), ActionContext::Searching, Action::EraseSearchChar),

    (key!(esc), ActionContext::Searching, Action::ClearSearch),

    (key!(alt-c),  ActionContext::None, Action::ChangeCaseSensitiveMode),
    (key!(ctrl-f), ActionContext::None, Action::ChangeGapSearchMode),

    (key!(ctrl-r), ActionContext::None, Action::RefreshListing),

    (key!('?'), ActionContext::None, Action::Help),

    (key!(esc),    ActionContext::NotSearching, Action::Exit),
    (key!(alt-q),  ActionContext::None, Action::Exit),
    (key!(ctrl-c), ActionContext::None, Action::ExitWithoutCd),

];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_default_keymap_keys_unique() {
        let mut key_counts: HashMap<(KeyEvent, ActionContext), usize> = HashMap::new();

        DEFAULT_KEYMAP
            .iter()
            .for_each(|(k, c, _)| *key_counts.entry((k.clone(), c.clone())).or_default() += 1);

        for (k, v) in key_counts {
            assert_eq!(v, 1, "found {} entries for key {:?} in context {:?}", v, k.0, k.1);
        }
    }

    #[test]
    fn check_all_actions_have_default_keymap() {
        let actions_in_default_keymap: Vec<Action> = DEFAULT_KEYMAP
            .iter()
            .map(|(k, c, a)| a.clone())
            .collect();
        for a in crate::ui::ALL_ACTIONS {
            assert!(actions_in_default_keymap.contains(a), "Action {:?} not found in default keymap", a)
        }
    }

    #[test]
    fn test_keyboard_mapping_cli_option1() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:Exit",
            ]);
        let settings = TereSettings::parse_cli_args(&m).unwrap();
        assert_eq!(settings.keymap.get(&(key!(ctrl-x), ActionContext::None)), Some(&Action::Exit));
    }

}
