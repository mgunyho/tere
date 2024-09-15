/// Module for managing the settings (command line arguments) of the app
use clap::{error::ErrorKind as ClapErrorKind, ArgMatches, Error as ClapError};
use crokey::key;
use crossterm::event::KeyEvent;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;
use strum_macros::EnumIter;

use crate::error::TereError;
use crate::ui::{Action, ActionContext};

//TODO: config file?

/// How to handle files while searching
#[derive(Debug, PartialEq, Eq)]
pub enum FileHandlingMode {
    /// Display files in the listing, but ignore them while searching (i.e. search only folders).
    /// This is the default behavior.
    Ignore,
    /// Hide files in the listing, only show folders.
    Hide,
    /// Match both files and folders while searching. Note that currently `tere` doesn't do anything with
    /// files, so matching a file just prints an error message.
    Match,
}

impl FileHandlingMode {
    /// When no file matches the search, display this message
    pub fn no_matches_message(&self) -> &'static str {
        match self {
            FileHandlingMode::Ignore => "No folders matching search",
            FileHandlingMode::Hide | FileHandlingMode::Match => "No matches",
        }
    }
}

impl Default for FileHandlingMode {
    fn default() -> Self {
        Self::Ignore
    }
}

#[derive(Debug, PartialEq, Eq)]
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
        write!(f, "{text}")
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum GapSearchMode {
    NormalSearch,
    NormalSearchAnywhere,
    GapSearchFromStart,
    GapSearchAnywhere,
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
            GapSearchMode::NormalSearch       => "normal search",
            GapSearchMode::NormalSearchAnywhere => "normal search anywhere",
            GapSearchMode::GapSearchAnywhere  => "gap search anywhere",
        };
        write!(f, "{text}")
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone, EnumIter, clap::ValueEnum)]
pub enum SortMode {
    Name,
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
            SortMode::Name     => "name",
            SortMode::Created  => "cre",
            SortMode::Modified => "mod",
        };
        write!(f, "{text}")
    }
}

#[derive(Default)]
pub struct TereSettings {
    /// How to handle files: Ignore, hide or match them.
    pub file_handling_mode: FileHandlingMode,
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

    pub skip_first_run_prompt: bool,
}

pub type DeprecationWarnings = Vec<&'static str>;

impl TereSettings {
    pub fn parse_cli_args(args: &ArgMatches) -> Result<(Self, DeprecationWarnings), TereError> {
        let mut ret = Self::default();
        let mut warnings: DeprecationWarnings = vec![];

        ret.file_handling_mode = match args
            .get_one::<String>("files")
            // ok to unwrap because files has a default value which is always present
            .unwrap()
            .as_str()
        {
            "ignore" | "i" => FileHandlingMode::Ignore,
            "hide" | "h" => FileHandlingMode::Hide,
            "match" | "m" => FileHandlingMode::Match,
            _ => unreachable!(),
        };

        // read these deprecated values afterwards, because otherwise the default value from
        // --files will override these
        if args.get_flag("folders-only") {
            ret.file_handling_mode = FileHandlingMode::Hide;
            warnings.push("The option '--folders-only' / '-d' has been deprecated, please use '--files hide' instead.")
        }

        if args.get_flag("no-folders-only") {
            ret.file_handling_mode = FileHandlingMode::Ignore;
            warnings.push("The option '--no-folders-only' / '-D' has been deprecated, please use '--files ignore' or '--files match' instead.")
        }

        if args.get_flag("filter-search") {
            ret.filter_search = true;
        }

        if args.get_flag("case-sensitive") {
            ret.case_sensitive = CaseSensitiveMode::CaseSensitive;
        } else if args.get_flag("ignore-case") {
            ret.case_sensitive = CaseSensitiveMode::IgnoreCase;
        } else if args.get_flag("smart-case") {
            ret.case_sensitive = CaseSensitiveMode::SmartCase;
        }

        if args.get_flag("gap-search") {
            ret.gap_search_mode = GapSearchMode::GapSearchFromStart;
        } else if args.get_flag("gap-search-anywhere") {
            ret.gap_search_mode = GapSearchMode::GapSearchAnywhere;
        } else if args.get_flag("normal-search") {
            ret.gap_search_mode = GapSearchMode::NormalSearch;
        } else if args.get_flag("normal-search-anywhere") {
            ret.gap_search_mode = GapSearchMode::NormalSearchAnywhere;
        } else if args.get_flag("no-gap-search") {
            warnings.push("The option '--no-gap-search' has been renamed to '--normal-search', please use that instead.");
            ret.gap_search_mode = GapSearchMode::NormalSearch;
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
                        format!("Invalid value for 'autocd-timeout': '{x}'\n"),
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
        if args.get_one::<String>("mouse").unwrap() == "on" {
            ret.mouse_enabled = true;
        }

        if let Some(false) = args.get_one::<bool>("clear-default-keymap") {
            ret.keymap = DEFAULT_KEYMAP
                .iter()
                // (*k).into() converts crokey KeyCombinaton to crossterm KeyEvent
                .map(|(k, c, a)| (((*k).into(), c.clone()), a.clone()))
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
                ClapErrorKind::InvalidValue,
                "No keyboard mapping found for exit!\n",
            )
            .into());
        }

        ret.sort_mode = args
            .get_one::<SortMode>("sort")
            .cloned()
            .unwrap_or_default();

        if args.get_flag("skip-first-run-prompt") {
            ret.skip_first_run_prompt = true;
        }

        Ok((ret, warnings))
    }
}

fn parse_keymap_arg(arg: &str) -> Result<Vec<(KeyEvent, ActionContext, Action)>, ClapError> {
    let mappings = arg.split(',');
    let mut ret = Vec::new();

    fn parsekey_to_clap(mapping: &str, err: crokey::ParseKeyError) -> ClapError {
        ClapError::raw(
            ClapErrorKind::InvalidValue,
            format!("Error parsing key combination '{mapping}': {err}\n"),
        )
    }

    fn strum_to_clap(mapping: &str, attempted_value: &str, ctx_or_action: &str) -> ClapError {
        ClapError::raw(
            ClapErrorKind::InvalidValue,
            format!("Error parsing key mapping '{mapping}': invalid {ctx_or_action} '{attempted_value}'\n"),
        )
    }

    for mapping in mappings {
        if mapping.is_empty() {
            return Err(ClapError::raw(
                ClapErrorKind::InvalidValue,
                format!("Invalid mapping: '{arg}'\n"),
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

        ret.push((k.into(), c, a));
    }

    Ok(ret)
}

// NOTE: can't create a const hashmap (without an extra dependency like phf), so just using a slice
// of tuples.
// NOTE: The key combinations are saved as crokey KeyCombinations, and converted to crossterm
// KeyEvents when this array is read during initialization
pub const DEFAULT_KEYMAP: &[(crokey::KeyCombination, ActionContext, Action)] = &[

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
    (key!(shift-'?'), ActionContext::None, Action::Help),

    (key!(esc),    ActionContext::NotSearching, Action::Exit),
    (key!(alt-q),  ActionContext::None, Action::Exit),
    (key!(ctrl-c), ActionContext::None, Action::ExitWithoutCd),

];

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function for creating TereSettings from cli args
    fn parse_cli(args: Vec<&str>) -> (TereSettings, DeprecationWarnings) {
        let m = crate::cli_args::get_cli_args().try_get_matches_from(args).unwrap();
        return TereSettings::parse_cli_args(&m).unwrap();
    }

    /// Helper for parsing cli args which should produce no deprecation warnings
    fn parse_cli_no_warnings(args: Vec<&str>) -> TereSettings {
        let (settings, warnings) = parse_cli(args);
        assert!(warnings.is_empty());
        settings
    }

    #[test]
    fn check_default_keymap_keys_unique() {
        let mut key_counts: HashMap<(KeyEvent, ActionContext), usize> = HashMap::new();

        DEFAULT_KEYMAP
            .iter()
            .for_each(|(k, c, _)| *key_counts.entry(((*k).into(), c.clone())).or_default() += 1);

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
        assert_eq!(e, &key!(ctrl-x).into());
        assert_eq!(c, &ActionContext::None);
        assert_eq!(a, &Action::Exit);
    }

    #[test]
    fn test_parse_keymap_arg2() {
        let m = parse_keymap_arg("ctrl-x:Exit,ctrl-j:NotSearching:CursorUp").unwrap();
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].0, key!(ctrl-x).into());
        assert_eq!(m[0].1, ActionContext::None);
        assert_eq!(m[0].2, Action::Exit);
        assert_eq!(m[1].0, key!(ctrl-j).into());
        assert_eq!(m[1].1, ActionContext::NotSearching);
        assert_eq!(m[1].2, Action::CursorUp);
    }

    #[test]
    fn test_keyboard_mapping_cli_option1() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
            "-m", "ctrl-x:Exit",
        ]);
        assert_eq!(settings.keymap.get(&(key!(ctrl-x).into(), ActionContext::None)), Some(&Action::Exit));
    }

    #[test]
    fn test_keyboard_mapping_cli_option2() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
            "-m", "ctrl-x:Exit,ctrl-y:ClearSearch",
        ]);
        assert_eq!(settings.keymap.get(&(key!(ctrl-x).into(), ActionContext::None)), Some(&Action::Exit));
        assert_eq!(settings.keymap.get(&(key!(ctrl-y).into(), ActionContext::None)), Some(&Action::ClearSearch));
    }

    #[test]
    fn test_keyboard_mapping_cli_option3() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
            "-m", "ctrl-x:Exit,ctrl-x:ClearSearch", // repeated mapping
        ]);
        assert_eq!(settings.keymap.get(&(key!(ctrl-x).into(), ActionContext::None)), Some(&Action::ClearSearch));
    }

    #[test]
    fn test_keyboard_mapping_cli_option4() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
            // provide option multiple times
            "-m", "ctrl-x:Exit",
            "-m", "ctrl-x:ClearSearch",
        ]);
        assert_eq!(settings.keymap.get(&(key!(ctrl-x).into(), ActionContext::None)), Some(&Action::ClearSearch));
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
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert_eq!(settings.keymap.get(&(key!(alt-h).into(), ActionContext::None)), Some(&Action::ChangeDirParent));
        assert_eq!(settings.keymap.get(&(key!(alt-j).into(), ActionContext::None)), Some(&Action::CursorDown));
        assert_eq!(settings.keymap.get(&(key!(alt-k).into(), ActionContext::None)), Some(&Action::CursorUp));
        assert_eq!(settings.keymap.get(&(key!(alt-l).into(), ActionContext::None)), Some(&Action::ChangeDir));

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "-m", "alt-h:None,alt-j:None,alt-k:None,alt-l:None",
        ]);
        assert_eq!(settings.keymap.get(&(key!(alt-h).into(), ActionContext::None)), None);
        assert_eq!(settings.keymap.get(&(key!(alt-j).into(), ActionContext::None)), None);
        assert_eq!(settings.keymap.get(&(key!(alt-k).into(), ActionContext::None)), None);
        assert_eq!(settings.keymap.get(&(key!(alt-l).into(), ActionContext::None)), None);
    }

    #[test]
    fn test_unmap2() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::NotSearching)), Some(&Action::Exit));
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::Searching)), Some(&Action::ClearSearch));
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::None)), None);

        assert_eq!(settings.keymap.get(&(key!(backspace).into(), ActionContext::Searching)), Some(&Action::EraseSearchChar));
        assert_eq!(settings.keymap.get(&(key!(backspace).into(), ActionContext::NotSearching)), Some(&Action::ChangeDirParent));
        assert_eq!(settings.keymap.get(&(key!(backspace).into(), ActionContext::None)), None);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "-m", "esc:Searching:None",
            "-m", "backspace:None", // this shouldn't affect any of the mappings since they are context-dependent
            "-m", "backspace:None:None", // this shouldn't affect any of the mappings since they are context-dependent
        ]);
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::NotSearching)), Some(&Action::Exit));
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::Searching)), None);
        assert_eq!(settings.keymap.get(&(key!(esc).into(), ActionContext::None)), None);
    }

    #[test]
    fn test_clear_default_keymap() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--clear-default-keymap",
            "--map", "ctrl-x:Exit",
        ]);
        assert!(settings.keymap.len() == 1);
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

    #[test]
    fn test_filter_search_override() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert!(!settings.filter_search);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--filter-search",
            "--no-filter-search",
        ]);
        assert!(!settings.filter_search);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--filter-search",
            "--no-filter-search",
            "--filter-search",
        ]);
        assert!(settings.filter_search);
    }

    #[test]
    fn test_files_parse() {
        let settings = parse_cli_no_warnings(vec!["foo"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);

        let settings = parse_cli_no_warnings(vec!["foo", "--files", "ignore"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);
        let settings = parse_cli_no_warnings(vec!["foo", "--files", "i"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "ignore"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "i"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);

        let settings = parse_cli_no_warnings(vec!["foo", "--files", "hide"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
        let settings = parse_cli_no_warnings(vec!["foo", "--files", "h"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "hide"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "h"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
        let settings = parse_cli_no_warnings(vec!["foo", "-lhide"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
        let settings = parse_cli_no_warnings(vec!["foo", "-lh"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);

        let settings = parse_cli_no_warnings(vec!["foo", "--files", "match"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Match);
        let settings = parse_cli_no_warnings(vec!["foo", "--files", "m"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Match);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "match"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Match);
        let settings = parse_cli_no_warnings(vec!["foo", "-l", "m"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Match);
    }

    #[test]
    #[should_panic(expected = "InvalidValue")]
    fn test_files_missing() {
        parse_cli_no_warnings(vec!["foo", "--files"]);
    }

    #[test]
    #[should_panic(expected = "InvalidValue")]
    fn test_files_invalid() {
        parse_cli_no_warnings(vec!["foo", "--files", "xxx"]);
    }

    #[test]
    fn test_files_override() {
        let settings = parse_cli_no_warnings(vec!["foo", "--files", "hide", "--files", "ignore"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);

        let settings = parse_cli_no_warnings(vec!["foo", "--files", "hide", "--files", "ignore", "--files", "h"]);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);
    }

    #[test]
    fn test_folders_only_deprecated() {
        let (settings, warnings) = parse_cli(vec!["foo", "--folders-only"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'--folders-only' / '-d' has been deprecated, please use '--files hide' instead"));
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);

        let (settings, warnings) = parse_cli(vec!["foo", "-d"]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Hide);

        let (settings, warnings) = parse_cli(vec!["foo", "--no-folders-only"]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'--no-folders-only' / '-D' has been deprecated, please use '--files ignore' or '--files match' instead"));
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);

        let (settings, warnings) = parse_cli(vec!["foo", "-D"]);
        assert_eq!(warnings.len(), 1);
        assert_eq!(settings.file_handling_mode, FileHandlingMode::Ignore);
    }

    #[test]
    fn test_case_sensitive_mode_override() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert_eq!(settings.case_sensitive, CaseSensitiveMode::SmartCase);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--case-sensitive",
            "--ignore-case",
        ]);
        assert_eq!(settings.case_sensitive, CaseSensitiveMode::IgnoreCase);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--case-sensitive",
            "--ignore-case",
            "--case-sensitive",
        ]);
        assert_eq!(settings.case_sensitive, CaseSensitiveMode::CaseSensitive);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--case-sensitive",
            "--ignore-case",
            "--case-sensitive",
            "--smart-case",
        ]);
        assert_eq!(settings.case_sensitive, CaseSensitiveMode::SmartCase);
    }

    #[test]
    fn test_gap_search_mode_override() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert_eq!(settings.gap_search_mode, GapSearchMode::GapSearchFromStart);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--gap-search",
            "--gap-search-anywhere",
        ]);
        assert_eq!(settings.gap_search_mode, GapSearchMode::GapSearchAnywhere);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--gap-search",
            "--gap-search-anywhere",
            "--normal-search-anywhere",
        ]);
        assert_eq!(settings.gap_search_mode, GapSearchMode::NormalSearchAnywhere);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--gap-search",
            "--gap-search-anywhere",
            "--normal-search-anywhere",
            "--gap-search",
        ]);
        assert_eq!(settings.gap_search_mode, GapSearchMode::GapSearchFromStart);
    }

    #[test]
    fn test_no_gap_search_deprecated() {
        let (settings, warnings) = parse_cli(vec![
            "foo",
            "--no-gap-search"
        ]);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("'--no-gap-search' has been renamed"));
        assert!(settings.gap_search_mode == GapSearchMode::NormalSearch);
    }

    #[test]
    fn test_sort_mode_override() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert_eq!(settings.sort_mode, SortMode::Name);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--sort", "created",
        ]);
        assert_eq!(settings.sort_mode, SortMode::Created);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--sort",  "created",
            "--sort",  "name",
            "--sort",  "modified",
        ]);
        assert_eq!(settings.sort_mode, SortMode::Modified);

    }

    #[test]
    fn test_mouse_override() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert!(!settings.mouse_enabled);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--mouse", "off",
            "--mouse", "on",
        ]);
        assert!(settings.mouse_enabled);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--mouse",  "off",
            "--mouse",  "on",
            "--mouse",  "off",
        ]);
        assert!(!settings.mouse_enabled);

    }

    #[test]
    fn test_skip_first_run_prompt() {
        let settings = parse_cli_no_warnings(vec![
            "foo",
        ]);
        assert!(!settings.skip_first_run_prompt);

        let settings = parse_cli_no_warnings(vec![
            "foo",
            "--skip-first-run-prompt",
        ]);
        assert!(settings.skip_first_run_prompt);
    }

}
