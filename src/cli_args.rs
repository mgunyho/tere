/// The CLI options for tere

use clap::{App, Arg};

macro_rules! case_sensitive_template {
    ($x:tt, $y:tt) => {
        format!("This overrides the --{} and --{} options. You can also change the case sensitivity mode while the program is running with the keyboard shortcut ALT+C.", $x, $y)
    }
}

macro_rules! gap_search_mode_template {
    ($x:tt, $y:tt) => {
        format!("This overrides the --{} and --{} options. You can also change the search mode while the program is running with the keyboard shortcut CTRL+F.", $x, $y)
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
             .long_help(format!("Enable case-sensitive search.\n\n{}",
                                case_sensitive_template!("ignore-case", "smart-case")).as_str())
             .overrides_with_all(&["ignore-case", "smart-case", "case-sensitive"])
            )
        .arg(Arg::new("ignore-case")
             .long("ignore-case")
             .short('i') // same as ripgrep
             .help("Ignore case when searching")
             .long_help(format!("Enable case-insensitive search.\n\n{}",
                                case_sensitive_template!("case-sensitive", "smart-case")).as_str())
             .overrides_with_all(&["smart-case", "ignore-case"])
            )
        .arg(Arg::new("smart-case")
             .long("smart-case")
             .short('S') // same as ripgrep
             .help("Smart case search (default)")
             .long_help(format!("Enable smart-case search. If the search query contains only lowercase letters, search case insensitively. Otherwise search case sensitively. This is the default search mode.\n\n{}",
                                case_sensitive_template!("case-sensitive", "ignore-case")).as_str())
             .overrides_with("smart-case")
            )
        .arg(Arg::new("gap-search")
             .long("gap-search")
             .short('g')
             .help("Match the search from the beginning, but allow gaps (default)")
             .long_help(format!("When searching, match items that start with the same character as the search query, but allow gaps between the search characters. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"dt\" would match \"DeskTop\" and \"DocumenTs\" but not \"downloads\", and searching for \"es\" would match none of the above. This is the default behavior.\n\n{}", gap_search_mode_template!("gap-search", "no-gap-search")).as_str())
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("gap-search-anywhere")
             .long("gap-search-anywhere")
             .short('G')
             .help("Match the search anywhere, and allow gaps")
             .long_help(format!("When searching, allow the search characters to appear anywhere in a file/folder name, possibly with gaps between them. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"es\" would match \"dESktop\" and \"documEntS\", but not \"downloads\".\n\n{}",
                                gap_search_mode_template!("gap-search-from-start", "no-gap-search")).as_str())
             .overrides_with_all(&["gap-search-anywhere", "no-gap-search"])
            )
        .arg(Arg::new("no-gap-search")
             .long("no-gap-search")
             .short('n')
             .help("Match the search from the beginning, and do not allow gaps")
             .long_help(format!("Disable gap-search. Match only consecutive characters from the beginning of the search query. For example, searching for \"do\" would match \"DOcuments\" and \"DOwnloads\", but not \"desktop\".\n\n{}", gap_search_mode_template!("gap-search", "gap-search-from-start")).as_str())
             .overrides_with("no-gap-search")
            )
        //TODO: if somebody wants this: '-N', '--no-gap-search-anywhere - don't allow gaps, but can start anywhere. maybe have to come up with a better long name.
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
