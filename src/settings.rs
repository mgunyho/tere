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

#[derive(PartialEq, Eq)]
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

pub enum SortMode {
    Name,
    Accessed,
    Created,
    Modified,
}

impl Default for SortMode {
    fn default() -> Self {
        Self::Name
    }
}

impl fmt::Display for SortMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            SortMode::Name         => "name",
            SortMode::Accessed => "acc",
            SortMode::Created => "cre",
            SortMode::Modified => "mod",
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

    pub sort_mode: SortMode,

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

        if args.contains_id("folders-only") {
            ret.folders_only = true;
        }

        if args.contains_id("filter-search") {
            ret.filter_search = true;
        }

        if args.contains_id("case-sensitive") {
            ret.case_sensitive = CaseSensitiveMode::CaseSensitive;
        } else if args.contains_id("ignore-case") {
            ret.case_sensitive = CaseSensitiveMode::IgnoreCase;
        } else if args.contains_id("smart-case") {
            ret.case_sensitive = CaseSensitiveMode::SmartCase;
        }

        if args.contains_id("gap-search") {
            ret.gap_search_mode = GapSearchMode::GapSearchFromStart;
        } else if args.contains_id("gap-search-anywhere") {
            ret.gap_search_mode = GapSearchMode::GapSearchAnywere;
        } else if args.contains_id("no-gap-search") {
            ret.gap_search_mode = GapSearchMode::NoGapSearch;
        }

        ret.autocd_timeout = match args
            .get_many::<String>("autocd-timeout")
            // ok to unwrap because autocd-timeout has a default value which is always present
            .unwrap()
            .map(|v| v.as_str())
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

        if let Some(hist_file) = args.get_one::<String>("history-file") {
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
        if args.get_many::<String>("mouse").unwrap().map(|v| v.as_str()).last().unwrap() == "on" {
            ret.mouse_enabled = true;
        }

        if !args.is_present("clear-default-keymap") {
            ret.keymap = DEFAULT_KEYMAP
                .iter()
                .map(|(k, c, a)| ((*k, c.clone()), a.clone()))
                .collect();
        }

        if let Some(mapping_args) = args.get_many("map") {
            for mapping_arg in mapping_args.cloned() {
                let mapping_arg: String = mapping_arg; // to enforce correct type coming from get_many
                let mappings = parse_keymap_arg(&mapping_arg)?;
                for (k, c, a) in mappings {
                    if a == Action::None {
                        ret.keymap.remove(&(k, c));
                    } else {
                        ret.keymap.insert((k, c), a);
                    }
                }
            }
        }

        if !ret.keymap.values().any(|a| a == &Action::Exit) {
            return Err(ClapError::raw(
                ClapErrorKind::EmptyValue,
                "No keyboard mapping found for exit!\n",
            ));
        }

        ret.sort_mode = match args
            .get_many::<String>("sort")
            .unwrap()
            .map(|v| v.as_str())
            .last()
            .unwrap()
        {

            "adate" => SortMode::Accessed,
            "cdate" => SortMode::Created,
            "mdate" => SortMode::Modified,
            "name"  => SortMode::Name,
            _       => unreachable!(),
        };

        Ok(ret)
    }
}

fn parse_keymap_arg(arg: &str) -> Result<Vec<(KeyEvent, ActionContext, Action)>, ClapError> {
    let mappings = arg.split(',');
    let mut ret = Vec::new();

    fn parsekey_to_clap(mapping: &str, err: crokey::ParseKeyError) -> ClapError {
        ClapError::raw(
            ClapErrorKind::InvalidValue,
            format!("Error parsing key combination '{}': {}\n", mapping, err),
        )
    }

    fn strum_to_clap(mapping: &str, attempted_value: &str, ctx_or_action: &str) -> ClapError {
        ClapError::raw(
            ClapErrorKind::InvalidValue,
            format!(
                "Error parsing key mapping '{}': invalid {} '{}'\n",
                mapping, ctx_or_action, attempted_value,
            ),
        )
    }

    for mapping in mappings {
        if mapping.is_empty() {
            return Err(ClapError::raw(
                ClapErrorKind::InvalidValue,
                format!("Invalid mapping: '{}'\n", arg),
            ));
        }

        //TODO: what if I want to map colon? see how crokey does the hyphen parsing
        let parts: Vec<&str> = mapping.split(':').collect();
        let (k, c, a) = match parts[..] {
            [keys, action] => (
                crokey::parse(keys).map_err(|e| parsekey_to_clap(mapping, e))?,
                ActionContext::None,
                Action::from_str(action).map_err(|_| strum_to_clap(mapping, action, "action"))?
            ),
            [keys, ctx, action] => (
                crokey::parse(keys).map_err(|e| parsekey_to_clap(mapping, e))?,
                ActionContext::from_str(ctx).map_err(|_| strum_to_clap(mapping, ctx, "context"))?,
                Action::from_str(action).map_err(|_| strum_to_clap(mapping, action, "action"))?
            ),
            _ => return Err(ClapError::raw(
                    ClapErrorKind::InvalidValue,
                    format!("Keyboard mapping is not of the form 'key-combination:action' or 'key-combination:context:action': '{}'\n", &mapping),
                    ))
        };

        ret.push((k, c, a));
    }

    Ok(ret)
}

// NOTE: can't create a const hashmap (without an extra dependency like phf), so just using a slice
// of tuples.
pub const DEFAULT_KEYMAP: &[(KeyEvent, ActionContext, Action)] = &[

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

    (key!(pageup),  ActionContext::None, Action::CursorUpScreen),
    (key!(alt-u),   ActionContext::None, Action::CursorUpScreen),
    (key!(ctrl-u),  ActionContext::None, Action::CursorUpScreen),

    (key!(pagedown), ActionContext::None, Action::CursorDownScreen),
    (key!(alt-d),    ActionContext::None, Action::CursorDownScreen),
    (key!(ctrl-d),   ActionContext::None, Action::CursorDownScreen),

    (key!(home),        ActionContext::None, Action::CursorTop),
    (key!(alt-g),       ActionContext::None, Action::CursorTop), // like vim 'gg'
    (key!(end),         ActionContext::None, Action::CursorBottom),
    (key!(alt-shift-g), ActionContext::None, Action::CursorBottom), // like vim 'G'

    (key!(backspace), ActionContext::Searching, Action::EraseSearchChar),

    (key!(esc), ActionContext::Searching, Action::ClearSearch),

    (key!(alt-f),  ActionContext::None, Action::ChangeFilterSearchMode),
    (key!(alt-c),  ActionContext::None, Action::ChangeCaseSensitiveMode),
    (key!(ctrl-f), ActionContext::None, Action::ChangeGapSearchMode),
    (key!(alt-s),  ActionContext::None, Action::ChangeSortMode),

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
            .for_each(|(k, c, _)| *key_counts.entry((*k, c.clone())).or_default() += 1);

        for (k, v) in key_counts {
            assert_eq!(v, 1, "found {} entries for key {:?} in context {:?}", v, k.0, k.1);
        }
    }

    #[test]
    fn check_all_actions_have_default_keymap() {
        use strum::IntoEnumIterator;

        let actions_in_default_keymap: Vec<Action> = DEFAULT_KEYMAP
            .iter()
            .map(|(_, _, a)| a.clone())
            .collect();
        for a in Action::iter() {
            if a != Action::None {
                assert!(actions_in_default_keymap.contains(&a), "Action {:?} not found in default keymap", a)
            }
        }
    }

    #[test]
    fn test_parse_keymap_arg1() {
        let m = parse_keymap_arg("ctrl-x:Exit").unwrap();
        assert_eq!(m.len(), 1);
        let (e, c, a) = &m[0];
        assert_eq!(e, &key!(ctrl-x));
        assert_eq!(c, &ActionContext::None);
        assert_eq!(a, &Action::Exit);
    }

    #[test]
    fn test_parse_keymap_arg2() {
        let m = parse_keymap_arg("ctrl-x:Exit,ctrl-j:NotSearching:CursorUp").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].0, key!(ctrl-x));
        assert_eq!(m[0].1, ActionContext::None);
        assert_eq!(m[0].2, Action::Exit);
        assert_eq!(m[1].0, key!(ctrl-j));
        assert_eq!(m[1].1, ActionContext::NotSearching);
        assert_eq!(m[1].2, Action::CursorUp);
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

    #[test]
    fn test_keyboard_mapping_cli_option2() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:Exit,ctrl-y:ClearSearch",
            ]);
        let settings = TereSettings::parse_cli_args(&m).unwrap();
        assert_eq!(settings.keymap.get(&(key!(ctrl-x), ActionContext::None)), Some(&Action::Exit));
        assert_eq!(settings.keymap.get(&(key!(ctrl-y), ActionContext::None)), Some(&Action::ClearSearch));
    }

    #[test]
    fn test_keyboard_mapping_cli_option3() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:Exit,ctrl-x:ClearSearch", // repeated mapping
            ]);
        let settings = TereSettings::parse_cli_args(&m).unwrap();
        assert_eq!(settings.keymap.get(&(key!(ctrl-x), ActionContext::None)), Some(&Action::ClearSearch));
    }

    #[test]
    fn test_keyboard_mapping_cli_option4() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                // provide option multiple times
                "-m", "ctrl-x:Exit",
                "-m", "ctrl-x:ClearSearch",
            ]);
        let settings = TereSettings::parse_cli_args(&m).unwrap();
        assert_eq!(settings.keymap.get(&(key!(ctrl-x), ActionContext::None)), Some(&Action::ClearSearch));
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong1() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:Exxit", // incorrect action
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong2() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-:Exit", // inccorect mapping
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong3() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:Wrong:Exit", // incorrect context
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong4() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x::Exit", // Incorrect syntax
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong5() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x", // missing mapping and/or context
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_keyboard_mapping_cli_option_wrong6() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "ctrl-x:", // missing mapping and/or context
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_unmap1() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
            ]);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-h), ActionContext::None)), Some(&Action::ChangeDirParent));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-j), ActionContext::None)), Some(&Action::CursorDown));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-k), ActionContext::None)), Some(&Action::CursorUp));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-l), ActionContext::None)), Some(&Action::ChangeDir));

        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "alt-h:None,alt-j:None,alt-k:None,alt-l:None",
            ]);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-h), ActionContext::None)), None);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-j), ActionContext::None)), None);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-k), ActionContext::None)), None);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(alt-l), ActionContext::None)), None);
    }

    #[test]
    fn test_unmap2() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
            ]);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::NotSearching)), Some(&Action::Exit));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::Searching)), Some(&Action::ClearSearch));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::None)), None);

        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(backspace), ActionContext::Searching)), Some(&Action::EraseSearchChar));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(backspace), ActionContext::NotSearching)), Some(&Action::ChangeDirParent));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(backspace), ActionContext::None)), None);

        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "-m", "esc:Searching:None",
                "-m", "backspace:None", // this shouldn't affect any of the mappings since they are context-dependent
                "-m", "backspace:None:None", // this shouldn't affect any of the mappings since they are context-dependent
            ]);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::NotSearching)), Some(&Action::Exit));
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::Searching)), None);
        assert_eq!(TereSettings::parse_cli_args(&m).unwrap().keymap.get(&(key!(esc), ActionContext::None)), None);
    }

    #[test]
    fn test_clear_default_keymap() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "--clear-default-keymap",
                "--map", "ctrl-x:Exit",
            ]);
        assert!(TereSettings::parse_cli_args(&m).unwrap().keymap.len() == 1);
    }

    #[test]
    fn test_empty_keymap_is_error() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "--clear-default-keymap",
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

    #[test]
    fn test_unmap_exit_is_error() {
        let m = crate::cli_args::get_cli_args()
            .get_matches_from(vec![
                "foo",
                "--map", "esc:NotSearching:None,alt-q:None",
            ]);
        assert!(TereSettings::parse_cli_args(&m).is_err());
    }

}
