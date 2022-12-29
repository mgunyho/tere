use clap::{Command, Arg, ArgAction};
use crate::ui::{Action, ActionContext};
use crate::settings::SortMode;
use strum::IntoEnumIterator;

/// The CLI options for tere

macro_rules! case_sensitive_template {
    ($help_text:tt, $x:tt, $y:tt) => {
        concat!(
            $help_text,
            "\n\nThis overrides the --", $x, " and --", $y,
            " options. You can also change the case sensitivity mode while the program is running with the keyboard shortcut Alt-c by default."
            )
    }
}

macro_rules! gap_search_mode_template {
    ($help_text:tt, $x:tt, $y:tt, $z:tt $(,)?) => {
        concat!(
            $help_text,
            "\n\nThis overrides the --", $x, ", --", $y, " and --", $z,
            " options. You can also change the search mode while the program is running with the keyboard shortcut Ctrl-f by default."
        )
    }
}

pub fn get_cli_args() -> Command<'static> {
    Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        //.author(env!("CARGO_PKG_AUTHORS")) // TODO: rest of these https://stackoverflow.com/a/27841363
        .global_setting(clap::AppSettings::DeriveDisplayOrder)
        .arg(Arg::new("filter-search")
             .action(ArgAction::SetTrue)
             .long("filter-search")
             //.visible_alias("fs") //TODO: consider
             .short('f')
             .help("Show only items matching the search in listing")
             .long_help("Show only items matching the current search query in the listing. This overrides the --no-filter-search option. You can toggle the filtering with the keyboard shortcut Alt-f by default.")
             .overrides_with_all(&["filter-search", "no-filter-search"])
            )
        .arg(Arg::new("no-filter-search")
             .action(ArgAction::SetTrue)
             .long("no-filter-search")
             //.visible_alias("nfs") //TODO: consider
             .short('F')
             .help("Show all items in the listing even when searching (default)")
             .long_help("Show all items in the listing even when searching (default). This overrides the --filter-search option. You can toggle the filtering with the keyboard shortcut Alt-f by default.")
             .overrides_with_all(&["filter-search", "no-filter-search"])
            )
        .arg(Arg::new("folders-only")
             .action(ArgAction::SetTrue)
             .long("folders-only")
             //.visible_alias("fo") //TODO: consider
             .short('d')
             .help("Show only folders in the listing")
             .long_help("Show only folders (and symlinks pointing to folders) in the listing. This overrides the --no-folders-only option.")
             .overrides_with_all(&["folders-only", "no-folders-only"])
            )
        .arg(Arg::new("no-folders-only")
             .action(ArgAction::SetTrue)
             .long("no-folders-only")
             //.visible_alias("nfo") //TODO: consider
             .short('D')
             .help("Show files and folders in the listing (default)")
             .long_help("Show both files and folders in the listing. This is the default view mode. This overrides the --folders-only option.")
             .overrides_with_all(&["folders-only", "no-folders-only"])
            )
        .arg(Arg::new("case-sensitive")
             .action(ArgAction::SetTrue)
             .long("case-sensitive")
             .short('s')  // same as ripgrep
             .help("Case sensitive search")
             .long_help(case_sensitive_template!(
                     "Enable case-sensitive search.",
                     "ignore-case",
                     "smart-case"
            ))
             .overrides_with_all(&["ignore-case", "smart-case", "case-sensitive"])
            )
        .arg(Arg::new("ignore-case")
             .action(ArgAction::SetTrue)
             .long("ignore-case")
             .short('i') // same as ripgrep
             .help("Ignore case when searching")
             .long_help(case_sensitive_template!(
                     "Enable case-insensitive search.",
                     "case-sensitive",
                     "smart-case"
                     ))
             .overrides_with_all(&["ignore-case", "smart-case", "case-sensitive"])
            )
        .arg(Arg::new("smart-case")
             .action(ArgAction::SetTrue)
             .long("smart-case")
             .short('S') // same as ripgrep
             .help("Smart case search (default)")
             .long_help(case_sensitive_template!(
                     "Enable smart-case search. If the search query contains only lowercase letters, search case insensitively. Otherwise search case sensitively. This is the default search mode.",
                     "case-sensitive",
                     "ignore-case"
                     ))
             .overrides_with_all(&["ignore-case", "smart-case", "case-sensitive"])
            )
        .arg(Arg::new("gap-search")
             .action(ArgAction::SetTrue)
             .long("gap-search")
             .short('g')
             .help("Match the search from the beginning, but allow gaps (default)")
             .long_help(gap_search_mode_template!(
                     "When searching, match items that start with the same character as the search query, but allow gaps between the search characters. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"dt\" would match \"DeskTop\" and \"DocumenTs\" but not \"downloads\", and searching for \"es\" would match none of the above. This is the default behavior.",
                     "gap-search-anywhere",
                     "normal-search",
                     "normal-search-anywhere",
                     ))
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "normal-search", "normal-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("gap-search-anywhere")
             .action(ArgAction::SetTrue)
             .long("gap-search-anywhere")
             .short('G')
             .help("Match the search anywhere, and allow gaps")
             .long_help(gap_search_mode_template!(
                     "When searching, allow the search characters to appear anywhere in a file/folder name, possibly with gaps between them. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"es\" would match \"dESktop\" and \"documEntS\", but not \"downloads\".",
                     "gap-search",
                     "normal-search",
                     "normal-search-anywhere",
                     ))
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "normal-search", "normal-search-anywhere", "no-gap-search"])
            )
        // DEPRECATED in favor of normal-search, this is here only for backward compatibility
        .arg(Arg::new("no-gap-search")
             .action(ArgAction::SetTrue)
             .long("no-gap-search")
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "normal-search", "normal-search-anywhere", "no-gap-search"])
             .hide(true)
            )
        .arg(Arg::new("normal-search")
             .action(ArgAction::SetTrue)
             .long("normal-search")
             .short('n')
             .help("Match the search from the beginning, and do not allow gaps")
             .long_help(gap_search_mode_template!(
                     "Match only consecutive characters, and only from the beginning of the folder or file name. For example, searching for \"do\" would match \"DOcuments\" and \"DOwnloads\", but not \"desktop\".",
                     "gap-search",
                     "gap-search-anywhere",
                     "normal-search-anywhere",
                     ))
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "normal-search", "normal-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("normal-search-anywhere")
             .action(ArgAction::SetTrue)
             .long("normal-search-anywhere")
             .short('N')
             .help("Match search anywhere, but do not allow gaps")
             .long_help(gap_search_mode_template!(
                     "Match only consecutive characters, but they may appear anywhere in file or folder name, not just the beginning. For example, searching for \"e\" would match \"documEnts\" and \"dEsktop\", but not \"downloads\".",
                     "gap-search",
                     "gap-search-anywhere",
                     "normal-search",
                     ))
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "normal-search", "normal-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("map")
             .action(ArgAction::Append)
             .long("map")
             .short('m')
             .help("Map one or more keyboard shortcuts. See full help (with --help) for further details.")
             // We need to provide static strings to the long help, but then we can't use format
             // because it returns String, and it won't live long enough. So we leak the string,
             // which is ok because it's done only once. Another option would be to create a
             // wrapper object to own the strings, but this is simpler.
             // see https://stackoverflow.com/questions/64184984/dynamically-generate-subcommands-with-formatted-description-in-clap
             // and https://stackoverflow.com/questions/65303960/clap-how-to-pass-a-default-value-when-returning-argmatchesstatic
             .long_help(&*Box::leak(format!(
"Add one or more keyboard shortcut mappings. The basic syntax is of the form 'key-combination:action' or 'key-combination:context:action', see examples below. This option can be provided multiple times, and multiple mappings can be created by a comma-separated list of mappings. If the same key combination (with the same context) is provided multiple times, the previous mappings are overridden. Use the action 'None' to remove a previously added mapping or one of the default mappings.

Examples:

    -m ctrl-x:Exit - Exit tere by typing ctrl-x
    -m ctrl-h:ChangeDirParent,ctrl-j:CursorDown,ctrl-k:CursorUp,ctrl-l:ChangeDir - Navigate using Control + hjkl in addition to the default Alt + hjkl
    -m 1:NotSearching:CursorTop - Move the cursor to the top of the listing by typing '1', but only if not already searching (so you can still search for filenames that contain '1')
    -m esc:NotSearching:ExitWithoutCd,enter:ChangeDirAndExit - Map Escape to exiting with error, and map Enter to select the directory under the cursor and exit
    -m alt-h:None,alt-j:None,alt-k:None,alt-l:None - Disable navigation using Alt+hjkl
    -m esc:Searching:None - Don't clear the search by pressing esc, but still exit using esc (if the search query is empty)

Possible actions:

{}

Possible contexts:

{}
",
justify_and_indent(
    &Action::iter().map(|a| a.to_string()).collect::<Vec<_>>(),
    &Action::iter().map(|a| a.description().to_string()).collect::<Vec<_>>()
    ),
justify_and_indent(
    &ActionContext::iter().map(|a| a.to_string()).collect::<Vec<_>>(),
    &ActionContext::iter().map(|a| a.description().to_string()).collect::<Vec<_>>()
    ),
).into_boxed_str()))
            .takes_value(true)
            .value_name("MAPPING")
            )
        .arg(Arg::new("clear-default-keymap")
             .action(ArgAction::SetTrue)
             .long("clear-default-keymap")
             .help("Do not use the default keyboard mapping.")
             .long_help("Do not use the default keyboard mapping, so that all shortcuts have to be manually created from scratch using the --map/-m option. If no mapping for Exit is provided, tere will not run.")
             )
        .arg(Arg::new("sort")
             .action(ArgAction::Set)
             .long("sort")
             .help("Select sorting mode")
             .long_help("Choose whether to sort the listing by name, or the time of creation or modification. You can change the sort order with the keyboard shortcut Alt-s by default.")
             .value_name("'name', 'created', or 'modified'")
             .value_parser(clap::builder::EnumValueParser::<SortMode>::new())
             .hide_possible_values(true)
             //NOTE: couldn't figure out how to get this using clap derive, even though SortMode implements Default. So have to have it here.
             .default_value("name")
            )
        .arg(Arg::new("autocd-timeout")
             .action(ArgAction::Set)
             .long("autocd-timeout")
             .help("Timeout for auto-cd when there's only one match, in milliseconds. Use 'off' to disable auto-cd.")
             .long_help("If the current search matches only one folder, automatically change to that folder after this many milliseconds. If the value is 'off', automatic cding is disabled, and you have to manually enter the folder. Setting the timeout to zero is not recommended, because it makes navigation confusing.")
             .default_value("200")
             .value_name("TIMEOUT or 'off'")
             .overrides_with("autocd-timeout")
            )
        .arg(Arg::new("history-file")
             .action(ArgAction::Set)
             .long("history-file")
             .help("Save history to the file at this absolute path. Set to empty to disable.")
             .long_help("Save a history of visited folders in this file in JSON format. Should be an absolute path. Set to empty to disable saving history. If not provided, defaults to '$CACHE_DIR/tere/history.json', where $CACHE_DIR is the cache directory, i.e. $XDG_CACHE_HOME or ~/.cache. Note that the history file reveals parts of your folder structure if it can be read by someone else.")
             .takes_value(true)
             .value_name("FILE or ''")
            )
        .arg(Arg::new("mouse")
             .action(ArgAction::Set)
             .long("mouse")
             .help("Enable mouse navigation")
             .long_help("Enable mouse navigation. If enabled, you can browse by clicking around with the mouse.")
             .takes_value(true)
             .value_name("'on' or 'off'")
             .value_parser(clap::builder::PossibleValuesParser::new(["on", "off"]))
             .hide_possible_values(true)
             .default_value("off")
            )
}

/// Justify the list of enum variants (i.e. `ALL_ACTIONS` or `ALL_CONTEXTS`) and their
/// descriptions, and indent them to be printed in the help text
fn justify_and_indent(variants: &[String], descriptions: &[String]) -> String {

    let indentation: String = " ".repeat(4);
    let max_len = variants.iter().map(|x| x.len()).max().expect("list of variants is empty");

    let lines: Vec<String> = variants.iter().zip(descriptions)
        .map(|(x, d)| indentation.clone() + x + &" ".repeat(max_len - x.len() + 2) + d)
        .collect();

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_cli() {
        get_cli_args().debug_assert();
    }
}
