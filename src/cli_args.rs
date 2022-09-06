use clap::{App, Arg};

/// The CLI options for tere

macro_rules! case_sensitive_template {
    ($help_text:tt, $x:tt, $y:tt) => {
        concat!(
            $help_text,
            "\n\nThis overrides the --", $x, " and --", $y,
            " options. You can also change the case sensitivity mode while the program is running with the keyboard shortcut ALT+C."
            )
    }
}

macro_rules! gap_search_mode_template {
    ($help_text:tt, $x:tt, $y:tt) => {
        concat!(
            $help_text,
            "\n\nThis overrides the --", $x, " and --", $y,
            " options. You can also change the search mode while the program is running with the keyboard shortcut CTRL+F."
        )
    }
}

pub fn get_cli_args() -> App<'static> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        //.author(env!("CARGO_PKG_AUTHORS")) // TODO: rest of these https://stackoverflow.com/a/27841363
        .global_setting(clap::AppSettings::DeriveDisplayOrder)
        .arg(Arg::new("filter-search")
             .long("filter-search")
             //.visible_alias("fs") //TODO: consider
             .short('f')
             .help("Show only items matching the search in listing")
             .long_help("Show only items matching the current search query in the listing. This overrides the --no-filter-search option.")
             .overrides_with("filter-search")
            )
        .arg(Arg::new("no-filter-search")
             .long("no-filter-search")
             //.visible_alias("nfs") //TODO: consider
             .short('F')
             .help("Show all items in the listing even when searching (default)")
             .long_help("Show all items in the listing even when searching (default). This overrides the --filter-search option.")
             .overrides_with_all(&["filter-search", "no-filter-search"])
            )
        .arg(Arg::new("folders-only")
             .long("folders-only")
             //.visible_alias("fo") //TODO: consider
             .short('d')
             .help("Show only folders in the listing")
             .long_help("Show only folders (and symlinks pointing to folders) in the listing. This overrides the --no-folders-only option.")
             .overrides_with("folders-only")
            )
        .arg(Arg::new("no-folders-only")
             .long("no-folders-only")
             //.visible_alias("nfo") //TODO: consider
             .short('D')
             .help("Show files and folders in the listing (default)")
             .long_help("Show both files and folders in the listing. This is the default view mode. This overrides the --folders-only option.")
             .overrides_with_all(&["folders-only", "no-folders-only"])
            )
        .arg(Arg::new("case-sensitive")
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
             .long("ignore-case")
             .short('i') // same as ripgrep
             .help("Ignore case when searching")
             .long_help(case_sensitive_template!(
                     "Enable case-insensitive search.",
                     "case-sensitive",
                     "smart-case"
                     ))
             .overrides_with_all(&["smart-case", "ignore-case"])
            )
        .arg(Arg::new("smart-case")
             .long("smart-case")
             .short('S') // same as ripgrep
             .help("Smart case search (default)")
             .long_help(case_sensitive_template!(
                     "Enable smart-case search. If the search query contains only lowercase letters, search case insensitively. Otherwise search case sensitively. This is the default search mode.",
                     "case-sensitive",
                     "ignore-case"
                     ))
             .overrides_with("smart-case")
            )
        .arg(Arg::new("gap-search")
             .long("gap-search")
             .short('g')
             .help("Match the search from the beginning, but allow gaps (default)")
             .long_help(gap_search_mode_template!(
                     "When searching, match items that start with the same character as the search query, but allow gaps between the search characters. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"dt\" would match \"DeskTop\" and \"DocumenTs\" but not \"downloads\", and searching for \"es\" would match none of the above. This is the default behavior.",
                     "gap-search",
                     "no-gap-search"
                     ))
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("gap-search-anywhere")
             .long("gap-search-anywhere")
             .short('G')
             .help("Match the search anywhere, and allow gaps")
             .long_help(gap_search_mode_template!(
                     "When searching, allow the search characters to appear anywhere in a file/folder name, possibly with gaps between them. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"es\" would match \"dESktop\" and \"documEntS\", but not \"downloads\".",
                     "gap-search-from-start",
                     "no-gap-search"
                     ))
             .overrides_with_all(&["gap-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("no-gap-search")
             .long("no-gap-search")
             .short('n')
             .help("Match the search from the beginning, and do not allow gaps")
             .long_help(gap_search_mode_template!(
                     "Disable gap-search. Match only consecutive characters from the beginning of the search query. For example, searching for \"do\" would match \"DOcuments\" and \"DOwnloads\", but not \"desktop\".",
                     "gap-search",
                     "gap-search-from-start"
                     ))
             .overrides_with("no-gap-search")
            )
        //TODO: if somebody wants this: '-N', '--no-gap-search-anywhere - don't allow gaps, but can start anywhere. maybe have to come up with a better long name.
        .arg(Arg::new("map")
             .long("map")
             .short('m')
             .help("Map one or more keyboard shortcuts. See full help (with --help) for further details.")
             .long_help(
"Add one or more keyboard shortcut mappings. The basic syntax is of the form 'key-combination:action' or 'key-combination:context:action', see examples below. This option can be provided multiple times, and multiple mappings can be created by a comma-separated list of mappings. If the same key combination (with the same context) is provided multiple times, the previous mappings are overridden. Use the action 'None' to remove a previously added mapping or one of the default mappings.

Examples:

    -m ctrl-x:Exit - Exit tere by typing ctrl-x
    -m ctrl-h:ChangeDirParent,ctrl-j:CursorDown,ctrl-k:CursorUp,ctrl-l:ChangeDir - Navigate using Control + hjkl in addition to the default Alt + hjkl.
    -m 1:NotSearching:CursorFirst - Move the cursor to the top of the listing by typing '1', but only if not already searching (so you can still search for filenames that contain the number '1')
    -m alt-h:None,alt-j:None,alt-k:None,alt-l:None - Disable navigation using alt+hjkl
    -m esc:Searching:None - Don't clear the search by pressing esc, but still exit using esc (if the search query is empty)

Possible actions:

?

Possible contexts:

    None - This mapping applies if no other context applies. This is the behavior if no context is specified in the mapping.
    Searching - This mapping only applies while searching (at least one search character has been given).
    NotSearching - This mapping only applies while not searching.
")
            .takes_value(true)
            .value_name("MAPPING")
            .multiple_occurrences(true)
            )
        .arg(Arg::new("clear-default-keymap")
             .long("clear-default-keymap")
             .help("Do not use the default keyboard mapping. Warning: if no mapping for Exit is provided, you will not be able to exit tere.")
             .long_help("Do not use the default keyboard mapping, so that all shortcuts have to be manually created from scratch using the -m/--map option. Warning: if no mapping for Exit is provided, you will not be able to exit tere.")
             )
        .arg(Arg::new("autocd-timeout")
             .long("autocd-timeout")
             .help("Timeout for auto-cd when there's only one match, in milliseconds. Use 'off' to disable auto-cd.")
             .long_help("If the current search matches only one folder, automatically change to that folder after this many milliseconds. If the value is 'off', automatic cding is disabled, and you have to manually enter the folder. Setting the timeout to zero is not recommended, because it makes navigation confusing.")
             .default_value("200")
             .value_name("TIMEOUT or 'off'")
             .overrides_with("autocd-timeout")
            )
        .arg(Arg::new("history-file")
             .long("history-file")
             .help("Save history to the file at this absolute path. Set to empty to disable.")
             .long_help("Save a history of visited folders in this file in JSON format. Should be an absolute path. Set to empty to disable saving history. If not provided, defaults to '$CACHE_DIR/tere/history.json', where $CACHE_DIR is the cache directory, i.e. $XDG_CACHE_HOME or ~/.cache. Note that the history file reveals parts of your folder structure if it can be read by someone else.")
             .takes_value(true)
             .value_name("FILE or ''")
            )
        .arg(Arg::new("mouse")
             .long("mouse")
             .help("Enable mouse navigation")
             .long_help("Enable mouse navigation. If enabled, you can browse by clicking around with the mouse.")
             .takes_value(true)
             .value_name("'on' or 'off'")
             .possible_values(&["on", "off"])
             .hide_possible_values(true)
             .default_value("off")
             .multiple_occurrences(true)
            )
}
