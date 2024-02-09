/// This module contains structs related to handling the application state,
/// independent of a "graphical" front-end, such as crossterm.

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::ffi::OsStr;
use std::io::{Error as IOError, ErrorKind, Result as IOResult};
use std::path::{Component as PathComponent, Path, PathBuf};
use std::fmt::Write as _;
use std::time::SystemTime;

use regex::Regex;

use crate::settings::{
    TereSettings,
    DeprecationWarnings,
    FileHandlingMode,
    CaseSensitiveMode,
    GapSearchMode,
    SortMode,
};

#[path = "history.rs"]
mod history;
use history::HistoryTree;

use crate::error::TereError;

/// The match locations of a given item. A list of *byte offsets* into the item's name that match
/// the current search pattern.
pub type MatchesLocType = Vec<(usize, usize)>;

/// A vector that keeps track of items that are 'filtered'. It offers indexing/viewing
/// both the vector of filtered items and the whole unfiltered vector.
pub struct MatchesVec {
    all_items: Vec<CustomDirEntry>,
    // Each key-value pair in this map corresponds to an item in `all_items` that matches the
    // current search. The key is the item's index in `all_items`, while the value contains the
    // regex match locations. We use a BTreeMap to always keep the matches sorted, so that they are
    // in the same order relative to each other as they are in `all_items`.
    matches: BTreeMap<usize, MatchesLocType>,
}

impl MatchesVec {
    /// Return a vector of the indices of the matches
    fn kept_indices(&self) -> Vec<usize> {
        self.matches.keys().copied().collect()
    }

    /// Return a vector of all items that have not been filtered out
    pub fn kept_items(&self) -> Vec<&CustomDirEntry> {
        self.matches
            .keys()
            .filter_map(|idx| self.all_items.get(*idx))
            .collect()
    }

    /// Update the collection of matching items by going through all items in the full collection
    /// and testing a regex pattern against the filenames
    pub fn update_matches(&mut self, search_ptn: &Regex, case_sensitive: bool, ignore_files: bool) {
        self.matches.clear();
        self.matches = self
            .all_items
            .iter()
            .enumerate()
            .filter_map(|(i, item)| {
                // if applicable, match only folders, not files
                if ignore_files && !item.is_dir() {
                    return None;
                }

                let target = if case_sensitive {
                    item.file_name_checked()
                } else {
                    item.file_name_checked().to_lowercase()
                };
                let mut capture_locations = search_ptn.capture_locations();
                if search_ptn
                    .captures_read(&mut capture_locations, &target)
                    .is_some()
                {
                    // have to do it this way using range because capture_locations has no iter() method
                    let locs = (1..capture_locations.len())
                        .filter_map(|i| capture_locations.get(i))
                        .collect();
                    Some((i, locs))
                } else {
                    None
                }
            })
            .collect();
    }
}

impl From<Vec<CustomDirEntry>> for MatchesVec {
    fn from(vec: Vec<CustomDirEntry>) -> Self {
        Self {
            all_items: vec,
            matches: BTreeMap::new(),
        }
    }
}

/// A stripped-down version of ``std::fs::DirEntry``.
#[derive(Clone)]
pub struct CustomDirEntry {
    _path: PathBuf,
    pub metadata: Option<std::fs::Metadata>,
    /// The symlink target is None if this entry is not a symlink
    pub symlink_target: Option<PathBuf>,
    _file_name: std::ffi::OsString,
}

impl CustomDirEntry {
    pub fn custom(path_buf: PathBuf) -> CustomDirEntry {
        Self{
            _path: path_buf.clone(),
            // TODO: Don't let this hack survive! But we need it to properly respond for `is_dir`
            metadata: Some(std::fs::metadata("/").unwrap()),
            symlink_target: None,
            _file_name: std::ffi::OsString::from(path_buf.clone()),
        }
    }

    /// Return the file name of this directory entry. The file name is an OsString,
    /// which may not be possible to convert to a String. In this case, this
    /// function returns an empty string.
    pub fn file_name_checked(&self) -> String {
        self._file_name.clone().into_string().unwrap_or_default()
    }

    pub fn path(&self) -> &PathBuf {
        &self._path
    }

    pub fn is_dir(&self) -> bool {
        match &self.metadata {
            Some(m) => m.is_dir(),
            None => false,
        }
    }

    pub fn created(&self) -> SystemTime {
        match &self.metadata {
            Some(m) => m.created().unwrap_or(SystemTime::UNIX_EPOCH),
            None => SystemTime::UNIX_EPOCH,
        }
    }

    pub fn modified(&self) -> SystemTime {
        match &self.metadata {
            Some(m) => m.modified().unwrap_or(SystemTime::UNIX_EPOCH),
            None => SystemTime::UNIX_EPOCH,
        }
    }
}

impl From<std::fs::DirEntry> for CustomDirEntry {
    fn from(e: std::fs::DirEntry) -> Self {
        Self {
            _path: e.path(),
            // Note: this traverses symlinks, so is_dir will return true for symlinks as well.
            metadata: std::fs::metadata(e.path()).ok(),
            symlink_target: std::fs::read_link(e.path()).ok(),
            _file_name: e.file_name(),
        }
    }
}

impl From<&Path> for CustomDirEntry {
    fn from(p: &Path) -> Self {
        Self {
            _path: p.to_path_buf(),
            metadata: p.metadata().ok(),
            symlink_target: p.read_link().ok(),
            _file_name: p.file_name().unwrap_or(p.as_os_str()).to_os_string(),
        }
    }
}

/// The type of the `ls_output_buf` buffer of the app state
pub type LsBufType = MatchesVec;

/// Possible non-error results of 'change directory' operation
#[derive(Debug)]
pub enum CdResult {
    /// The folder was changed successfully
    Success,

    /// Could not change to the desired directory, so changed to a folder that is one or more
    /// levels upwards.
    MovedUpwards {
        /// The (absolute) path where we were trying to cd
        target_abs_path: PathBuf,
        /// The error due to which we moved to a parent directory
        root_error: IOError,
    },
}

/// This struct represents the state of the application.
pub struct TereAppState {
    // Width and height of the main window. These values have to be updated by calling the
    // update_main_window_dimensions function if the window dimensions change.
    main_win_w: usize,
    main_win_h: usize,

    // This vector will hold the list of files/folders in the current directory,
    // including ".." (the parent folder).
    pub ls_output_buf: LsBufType,

    // Have to manually keep track of the logical absolute path of our app, see https://stackoverflow.com/a/70309860/5208725
    pub current_path: PathBuf,

    // The row on which the cursor is currently on, counted starting from the
    // top of the screen (not from the start of `ls_output_buf`). Note that this
    // doesn't have anything to do with the crossterm cursor position.
    pub cursor_pos: usize,

    // The top of the screen corresponds to this row in the `ls_output_buf`.
    pub scroll_pos: usize,

    search_string: String,

    pub header_msg: String,
    pub info_msg: String,

    _settings: TereSettings,

    history: HistoryTree,
}

impl TereAppState {
    /// Initialize the app state with the given settings. Note that the window dimensions are
    /// initialized to one, they need to be updated manually afterwards.
    pub fn init(settings: TereSettings, warnings: &DeprecationWarnings) -> Result<Self, TereError> {
        // Try to read the current folder from the PWD environment variable, since it doesn't have
        // symlinks resolved (this is what we want). If this fails for some reason (on windows?),
        // default to std::env::current_dir, which has resolved symlinks.
        let cwd = std::env::var("PWD")
            .map(PathBuf::from)
            .or_else(|_| std::env::current_dir())?;

        let info_msg = if warnings.is_empty() {
            format!(
                "{} {} - Type something to search, press '?' to view help or Esc to exit.",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION"),
            )
        } else {
            //TODO: red color or something?
            format!("Warning: {}", warnings.join(" "))
        };

        let mut ret = Self {
            main_win_w: 1,
            main_win_h: 1,
            ls_output_buf: vec![].into(),
            current_path: cwd.clone(),
            cursor_pos: 0,
            scroll_pos: 0,
            header_msg: "".into(),
            info_msg,
            search_string: "".into(),
            _settings: settings,
            history: HistoryTree::from_abs_path(cwd.clone()),
        };

        //read history tree from file, if applicable
        if let Some(hist_file) = &ret.settings().history_file {
            match std::fs::read_to_string(hist_file) {
                Ok(file_contents) => {
                    let mut tree: HistoryTree = serde_json::from_str(&file_contents)?;
                    tree.change_dir(cwd);
                    ret.history = tree;
                }
                Err(ref e) if e.kind() == ErrorKind::NotFound => {
                    // history file not created yet, no need to do anything
                }
                Err(e) => return Err(e.into()),
            }
        }

        ret.update_header();
        ret.update_ls_output_buf()?;

        ret.move_cursor(1, false); // start out from second entry, because first entry is '..'.
        if let Some(prev_dir) = ret.history.current_entry().last_visited_child_label() {
            ret.move_cursor_to_filename(prev_dir);
        }

        Ok(ret)
    }

    /// Things to do when the app is about to exit.
    pub fn on_exit(&self) -> IOResult<()> {
        if let Some(hist_file) = &self.settings().history_file {
            let parent_dir = hist_file.parent().ok_or_else(|| {
                IOError::new(ErrorKind::NotFound, "history file has no parent folder")
            })?;
            std::fs::DirBuilder::new()
                .recursive(true)
                .create(parent_dir)?;
            std::fs::write(hist_file, serde_json::to_string(&self.history)?)?;
        }
        Ok(())
    }

    ///////////////////////////////////////////
    // Helpers for reading the current state //
    ///////////////////////////////////////////

    pub fn settings(&self) -> &TereSettings {
        &self._settings
    }

    pub fn is_searching(&self) -> bool {
        !self.search_string.is_empty()
    }

    pub fn search_string(&self) -> &String {
        &self.search_string
    }

    /// The total number of items in the ls_output_buf.
    pub fn num_total_items(&self) -> usize {
        self.ls_output_buf.all_items.len()
    }

    /// The number of items that match the current search.
    pub fn num_matching_items(&self) -> usize {
        self.ls_output_buf.matches.len()
    }

    /// Return a vector that contains the indices into the currently visible
    /// items that contain a match
    pub fn visible_match_indices(&self) -> Vec<usize> {
        if self.is_searching() && self.settings().filter_search {
            (0..self.ls_output_buf.matches.len()).collect()
        } else {
            // it's ok to clone here, the kept_indices will be usually quite short.
            self.ls_output_buf.kept_indices()
        }
    }

    /// All items that are visible with the current settings in the current search state. This
    /// includes items that might fall outside the window.
    pub fn visible_items(&self) -> Vec<&CustomDirEntry> {
        if self.is_searching() && self.settings().filter_search {
            self.ls_output_buf.kept_items()
        } else {
            self.ls_output_buf.all_items.iter().collect()
        }
    }

    /// Shorthand to get the number of items without having to clone / iterate over all of them
    pub fn num_visible_items(&self) -> usize {
        if self.is_searching() && self.settings().filter_search {
            self.num_matching_items()
        } else {
            self.num_total_items()
        }
    }

    /// Convert a cursor position (in the range 0..window_height) to an index
    /// into the currently visible items.
    pub fn cursor_pos_to_visible_item_index(&self, cursor_pos: usize) -> usize {
        cursor_pos + self.scroll_pos
    }

    pub fn get_item_at_cursor_pos(&self, cursor_pos: usize) -> Option<&CustomDirEntry> {
        let idx = self.cursor_pos_to_visible_item_index(cursor_pos);
        self.visible_items().get(idx).copied()
    }

    /// Returns None if the visible items is empty, or if the state is
    /// inconsistent and the cursor is outside the currently visible items.
    fn get_item_under_cursor(&self) -> Option<&CustomDirEntry> {
        self.get_item_at_cursor_pos(self.cursor_pos)
    }

    /// Get the index of a filename into the currently visible items. Returns
    /// None if it's not found.
    fn index_of_filename<S: AsRef<OsStr>>(&self, fname: S) -> Option<usize> {
        self.visible_items()
            .iter()
            .position(|x| AsRef::<OsStr>::as_ref(&x.file_name_checked()) == fname.as_ref())
    }

    pub fn get_match_locations_at_cursor_pos(&self, cursor_pos: usize) -> Option<&MatchesLocType> {
        let idx = self.cursor_pos_to_visible_item_index(cursor_pos);
        if self.settings().filter_search {
            // NOTE: we assume that the matches is a sorted map
            self.ls_output_buf.matches.values().nth(idx)
        } else {
            self.ls_output_buf.matches.get(&idx)
        }
    }

    /// Perform an operation (op), while making sure that the cursor stays on the item where it
    /// was initially. Note: If `op` removes the previous item from the list of matches (i.e. list
    /// of valid cursor positions), this may leave the cursor position in an inconsistent state.
    fn with_cursor_fixed_at_current_item<F>(&mut self, op: F)
    where
        F: FnOnce(&mut Self),
    {
        let previous_item_under_cursor = self.get_item_under_cursor().cloned();
        op(self);
        previous_item_under_cursor.map(|itm| self.move_cursor_to_filename(itm.file_name_checked()));
    }

    //////////////////////////////////////
    // Functions for updating the state //
    //////////////////////////////////////

    pub fn update_header(&mut self) {
        self.header_msg = format!("{}", self.current_path.display());
    }

    pub fn update_main_window_dimensions(&mut self, w: usize, h: usize) {
        let delta_h = h.saturating_sub(self.main_win_h);
        self.main_win_w = w;
        self.main_win_h = h;
        self.move_cursor(0, false); // make sure that cursor is within view
        if delta_h > 0 {
            // height is increasing, scroll backwards as much as possible
            let old_scroll_pos = self.scroll_pos;
            self.scroll_pos = self.scroll_pos.saturating_sub(delta_h);
            self.cursor_pos += old_scroll_pos - self.scroll_pos;
        }
    }

    pub fn update_ls_output_buf(&mut self) -> IOResult<()> {
        let entries = std::fs::read_dir(&self.current_path)?;

        let mut entries: Box<dyn Iterator<Item = CustomDirEntry>> =
            Box::new(entries.filter_map(|e| e.ok()).map(CustomDirEntry::from));

        if self.settings().file_handling_mode == FileHandlingMode::Hide {
            entries = Box::new(entries.filter(|e| e.path().is_dir()));
        }

        let mut new_output_buf: Vec<CustomDirEntry> = entries.collect();

        new_output_buf.sort_by(|a, b| {
            match (a.is_dir(), b.is_dir()) {
                (true, true) | (false, false) => {
                    match &self.settings().sort_mode {
                        SortMode::Name => {
                            // both are dirs or files, compare by name.
                            // partial_cmp for strings always returns Some, so unwrap is ok here
                            a.file_name_checked()
                                .to_lowercase()
                                .partial_cmp(&b.file_name_checked().to_lowercase())
                                .unwrap()
                        }
                        SortMode::Created => {
                            // b > a for sorting most recently created first
                            b.created().partial_cmp(&a.created()).unwrap()
                        }
                        SortMode::Modified => {
                            // b > a for sorting most recently modified first
                            b.modified().partial_cmp(&a.modified()).unwrap()
                        }
                    }
                }
                // Otherwise, put folders first
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
            }
        });

        // Add the parent directory entry after sorting to make sure it's always first
        new_output_buf.insert(
            0,
            CustomDirEntry::from(AsRef::<Path>::as_ref(&PathComponent::ParentDir)),
        );

        self.ls_output_buf = new_output_buf.into();
        Ok(())
    }

    /// Change the current working directory. If `path` is empty, change to the item under the
    /// cursor, otherwise convert path to an absolute path and cd to it.
    pub fn change_dir(&mut self, path: &str) -> IOResult<CdResult> {
        // TODO: add option to use xdg-open (or similar) on files?
        // check out https://crates.io/crates/open
        // (or https://docs.rs/opener/0.4.1/opener/)
        let target_path = if path.is_empty() {
            //TODO: error here if result is empty?
            self.get_item_under_cursor()
                .map_or("".to_string(), |s| s.file_name_checked())
        } else {
            path.to_string()
        };

        let (target_path, cd_result) = self.find_valid_cd_target(&PathBuf::from(target_path))?;

        self.current_path = target_path;
        self.clear_search(); // if cd was successful, clear the search
        self.update_ls_output_buf()?;

        self.cursor_pos = 0;
        self.scroll_pos = 0;

        // current_path is always the absolute logical path, so we can just cd to it. This causes a
        // bit of extra work (the history tree has to go all the way from the root to the path
        // every time), but that's not too bad. The alernative option of using history_tree.go_up()
        // / visit() and handling the special cases where target_path is '..' or a relative path or
        // so on, would be much more complicated and would risk having the history tree and logical
        // path out of sync.
        self.history.change_dir(&self.current_path);

        // move cursor one position down, so we're not at '..' if we've entered a folder with no history
        self.move_cursor(1, false);
        if let Some(prev_dir) = self.history.current_entry().last_visited_child_label() {
            self.move_cursor_to_filename(prev_dir);
        }

        Ok(cd_result)
    }

    /// Given a target path, check if it's a valid cd target, and if it isn't, go up the folder
    /// tree until a valid (i.e. existing) folder is found
    fn find_valid_cd_target(&self, original_target: &Path) -> IOResult<(PathBuf, CdResult)> {
        let original_target_abs = if original_target.is_absolute() {
            original_target.to_path_buf()
        } else {
            normalize_path(&self.current_path.join(original_target))
        };

        let mut final_target = original_target_abs.as_ref();
        let mut result = CdResult::Success;

        loop {
            match self.check_can_change_dir(final_target) {
                Ok(p) => break Ok((p, result)),
                Err(e) => {
                    match e.kind() {
                        ErrorKind::NotFound | ErrorKind::PermissionDenied => {
                            final_target = final_target
                                .parent()
                                .unwrap_or(PathComponent::RootDir.as_ref());
                            match result {
                                CdResult::Success => {
                                    result = CdResult::MovedUpwards {
                                        target_abs_path: original_target_abs.clone(),
                                        root_error: e,
                                    };
                                }
                                CdResult::MovedUpwards { .. } => {}
                            }
                        }
                        // other kinds of errors we don't know how to deal with, pass them on
                        _ => return Err(e),
                    }
                }
            }
        }
    }

    /// Check if a path is a valid cd target, and if it is, return an absolute path to it
    fn check_can_change_dir(&self, target_path: &Path) -> IOResult<PathBuf> {
        let full_path = if target_path.is_absolute() {
            target_path.to_path_buf()
        } else {
            normalize_path(&self.current_path.join(target_path))
        };

        // try to read the dir, if this succeeds, it's a valid target for cd'ing.
        std::fs::read_dir(&full_path).map(|_| full_path)
    }

    /////////////////////////////////////////////
    // Functions for changing the app settings //
    /////////////////////////////////////////////

    /// Change the filter search mode, and ensure that the app state is valid after that
    pub fn set_filter_search(&mut self, filter_search: bool) {
        // Toggling filter search doesn't affect the current match, so we can use set_filter_search
        self.with_cursor_fixed_at_current_item(|self_| {
            self_._settings.filter_search = filter_search;
        });
    }

    pub fn set_case_sensitive(&mut self, case_sensitive: CaseSensitiveMode) {
        self._settings.case_sensitive = case_sensitive;
        // a bit of a hack, but this is the easiest way to force the cursor to be on a valid match
        // after changing the mode.
        // TODO: check if with_cursor_fixed_at_current_item + update_search_matches() is enough?
        self.advance_search("");
    }

    pub fn set_gap_search_mode(&mut self, gap_search_mode: GapSearchMode) {
        self._settings.gap_search_mode = gap_search_mode;
        self.advance_search(""); // hacky, see the comment above in set_case_sensitive
    }

    pub fn set_sort_mode(&mut self, sort_mode: SortMode) {
        self.with_cursor_fixed_at_current_item(|self_| {
            self_._settings.sort_mode = sort_mode;
            self_.update_ls_output_buf().ok();
            //TODO: should probably have a separate method for re-sorting the matches vector...
            self_.update_search_matches();
        });
    }

    /////////////////////////////////////
    // Functions for moving the cursor //
    /////////////////////////////////////

    /// Move the cursor up (positive amount) or down (negative amount) in the
    /// currently visible items, and update the scroll position as necessary
    pub fn move_cursor(&mut self, amount: isize, wrap: bool) {
        let old_cursor_pos = self.cursor_pos;
        let n_visible_items = self.visible_items().len();
        let max_cursor_pos = self.main_win_h - 1;
        let old_scroll_pos = self.scroll_pos;

        // pointer_pos: the global location of the cursor in ls_output_buf
        let old_pointer_pos: usize = old_cursor_pos + old_scroll_pos;

        let new_pointer_pos = if n_visible_items == 0 {
            old_pointer_pos
        } else {
            let pointer_pos_signed = isize::try_from(old_pointer_pos).unwrap_or(isize::MAX);
            let n_visible_signed = isize::try_from(n_visible_items).unwrap_or(isize::MAX);
            let result = pointer_pos_signed + amount;
            if wrap {
                usize::try_from(result.rem_euclid(n_visible_signed)).unwrap_or(usize::MAX)
            } else {
                usize::try_from(result.max(0).min(n_visible_signed - 1)).unwrap_or(usize::MAX)
            }
        };

        // update scroll position and calculate new cursor position
        if n_visible_items <= max_cursor_pos {
            // all items fit on screen, set scroll to 0
            self.scroll_pos = 0;
            self.cursor_pos = new_pointer_pos;
        } else if new_pointer_pos <= old_scroll_pos {
            // new cursor position is above screen, scroll up
            self.scroll_pos = new_pointer_pos;
            self.cursor_pos = 0;
        } else if new_pointer_pos >= old_scroll_pos + max_cursor_pos {
            // new cursor position is below screen, scroll down
            self.cursor_pos = max_cursor_pos;
            self.scroll_pos = new_pointer_pos.saturating_sub(max_cursor_pos);
        } else {
            // cursor stays within view, no need to change scroll position
            self.cursor_pos = new_pointer_pos.saturating_sub(self.scroll_pos);
        }
    }

    /// Move the cursor so that it is at the location `row` in the
    /// currently visible items, and update the scroll position as necessary
    pub fn move_cursor_to(&mut self, row: usize) {
        self.move_cursor(
            row as isize - self.cursor_pos as isize - self.scroll_pos as isize,
            false,
        );
    }

    /// Move cursor to the position of a given filename. If the filename was
    /// not found, don't move the cursor and return false, otherwise return true.
    pub fn move_cursor_to_filename<S: AsRef<OsStr>>(&mut self, fname: S) -> bool {
        self.index_of_filename(fname)
            .map(|idx| self.move_cursor_to(idx))
            .is_some()
    }

    /// Move the cursor to the next or previous match in the current list of
    /// matches. If dir is positive, move to the next match, if it's negative,
    /// move to the previous match, and if it's zero, move the cursor to the
    /// current match.
    pub fn move_cursor_to_adjacent_match(&mut self, dir: isize) {
        if self.is_searching() {
            if self.num_matching_items() == 0 {
                // if there are no matches, just move the cursor by one step
                self.move_cursor(dir.signum(), true);
                return;
            }

            if self.settings().filter_search {
                // the only visible items are the matches, so we can just move the cursor
                self.move_cursor(dir.signum(), true);
            } else {
                let cur_idx = self.cursor_pos_to_visible_item_index(self.cursor_pos);
                let kept_indices = &self.ls_output_buf.kept_indices();
                let (cur_idx_in_kept, cur_idx_in_all) = kept_indices
                    .iter()
                    .enumerate()
                    .find(|(_, i_in_all)| **i_in_all >= cur_idx)
                    // if we skipped everything, wrap around and return the first
                    // item in the kept indices. shouldn't panic, kept_indices
                    // shouldn't be empty based on the num_matching_items()
                    // check above.
                    .unwrap_or((0, &kept_indices[0]));

                #[allow(clippy::comparison_chain)] // I think this is easier to understand this way
                let new_row = if dir < 0 {
                    let i = cur_idx_in_kept
                        .checked_sub(1)
                        .unwrap_or(kept_indices.len() - 1);
                    kept_indices[i]
                } else if dir > 0 {
                    let i = (cur_idx_in_kept + 1) % kept_indices.len();
                    kept_indices[i]
                } else {
                    // dir == 0, just use the current index
                    *cur_idx_in_all
                };

                self.move_cursor_to(new_row);
            }
        }
    }

    ////////////
    // Search //
    ////////////

    fn update_search_matches(&mut self) {
        let is_case_sensitive = match self.settings().case_sensitive {
            CaseSensitiveMode::IgnoreCase => false,
            CaseSensitiveMode::CaseSensitive => true,
            CaseSensitiveMode::SmartCase => self.search_string.chars().any(|c| c.is_uppercase()),
        };
        let search_string = if is_case_sensitive {
            self.search_string.clone()
        } else {
            self.search_string.to_lowercase()
        };

        // TODO: construct regex pattern inside MatchesVec instead? - it relies now on capture
        // groups which are defined by the format!() parens here...
        let mut regex_str = "".to_string();
        let gap_search_mode = &self.settings().gap_search_mode;
        if gap_search_mode == &GapSearchMode::NormalSearch {
            let _ = write!(regex_str, "^({})", regex::escape(&search_string));
        } else if gap_search_mode == &GapSearchMode::NormalSearchAnywhere {
            let _ = write!(regex_str, "({})", regex::escape(&search_string));
        } else {
            // enable gap search. Add '^' to the regex to match only from the start if applicable.
            if gap_search_mode == &GapSearchMode::GapSearchFromStart {
                regex_str.push('^');
            }
            regex_str.push_str(
                &search_string
                    .chars()
                    .map(|c| format!("({})", regex::escape(&c.to_string())))
                    .collect::<Vec<String>>()
                    .join(".*?"),
            );
        }

        // ok to unwrap, we have escaped the regex above
        let search_ptn = Regex::new(&regex_str).unwrap();
        self.ls_output_buf.update_matches(
            &search_ptn,
            is_case_sensitive,
            self.settings().file_handling_mode == FileHandlingMode::Ignore,
        );
    }

    pub fn clear_search(&mut self) {
        self.with_cursor_fixed_at_current_item(|self_| self_.search_string.clear());
    }

    pub fn advance_search(&mut self, query: &str) {
        // Can't use with_cursor_fixed_at_current_item, because the current item might not be a
        // match any more after updating the search string.
        let previous_item_under_cursor = self.get_item_under_cursor().cloned();

        self.search_string.push_str(query);

        self.update_search_matches();

        if self.settings().filter_search {
            if let Some(item) = previous_item_under_cursor {
                if !self.move_cursor_to_filename(item.file_name_checked()) {
                    self.move_cursor_to(0);
                }
            }
        } else {
            self.move_cursor_to_adjacent_match(0);
        }
    }

    pub fn erase_search_char(&mut self) {
        let previous_item_under_cursor = self.get_item_under_cursor().cloned();

        if self.search_string.pop().is_some() {
            //TODO: keep cursor position when there were no matches? should somehow push cursor position onto some stack when advancing search.

            self.update_search_matches();

            if self.settings().filter_search {
                if let Some(item) = previous_item_under_cursor {
                    if !self.move_cursor_to_filename(item.file_name_checked()) {
                        self.move_cursor_to(0);
                    }
                }
            } else {
                self.move_cursor_to_adjacent_match(0);
            }
        };
    }
}

/// Normalize a path
/// NOTE: have to manually implement this since the std doesn't have that feature yet, as
/// of July 2023.
/// see:
/// - https://github.com/rust-lang/rfcs/issues/2208
/// - https://github.com/rust-lang/rust/issues/92750 (std::path::absolute)
/// - https://github.com/gdzx/rfcs/commit/3c69f787b5b32fb9c9960c1e785e5cabcc794238
/// - abs_path crate
/// - relative_path crate
/// This function is copy-pasted from cargo::util::paths::normalize_path, https://docs.rs/cargo-util/0.1.1/cargo_util/paths/fn.normalize_path.html, under the MIT license
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ PathComponent::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            PathComponent::Prefix(..) => unreachable!(),
            PathComponent::RootDir => {
                ret.push(component.as_os_str());
            }
            PathComponent::CurDir => {}
            PathComponent::ParentDir => {
                ret.pop();
            }
            PathComponent::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Create folders from a list of folder names in a temp dir.
    fn create_test_folders(tmp: &TempDir, folder_names: &Vec<&str>) {
        for folder_name in folder_names {
            let p = tmp.path().join(folder_name);
            std::fs::create_dir(p).unwrap();
        }

        // Check that the folders were actually created
        let expected_folders: std::collections::HashSet<String> =
            folder_names.iter().map(|s| s.to_string()).collect();
        let actual_folders: std::collections::HashSet<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|x| x.unwrap())
            .filter(|x| x.path().is_dir())
            .map(|x| x.file_name().into_string().unwrap())
            .collect();

        assert_eq!(expected_folders, actual_folders);
    }

    /// Create (empty) files from a list of file names in a temp dir
    fn create_test_files(tmp: &TempDir, file_names: &Vec<&str>) {
        for file_name in file_names {
            let p = tmp.path().join(file_name);
            std::fs::File::create(p).unwrap();
        }

        let expected_files: std::collections::HashSet<String> =
            file_names.iter().map(|s| s.to_string()).collect();
        let actual_files: std::collections::HashSet<String> = std::fs::read_dir(tmp.path())
            .unwrap()
            .map(|x| x.unwrap())
            .filter(|x| x.path().is_file())
            .map(|x| x.file_name().into_string().unwrap())
            .collect();

        assert_eq!(expected_files, actual_files);
    }

    /// Create files and folders from lists of file and folder names in a temp dir, and initialize
    /// a test state in the temp dir. Note that the cursor position for the state will be 1, since
    /// the first item is '..'.
    fn create_test_state_with_files_and_folders(
        tmp: &TempDir,
        win_h: usize,
        file_names: Vec<&str>,
        folder_names: Vec<&str>,
    ) -> TereAppState {
        create_test_files(tmp, &file_names);
        create_test_folders(tmp, &folder_names);

        let mut state = TereAppState {
            cursor_pos: 0,
            scroll_pos: 0,
            main_win_h: win_h,
            main_win_w: 10,
            current_path: "/".into(),
            ls_output_buf: vec![].into(),
            header_msg: "".into(),
            info_msg: "".into(),
            search_string: "".into(),
            _settings: Default::default(),
            history: HistoryTree::from_abs_path("/"),
        };
        state.change_dir(tmp.path().to_str().unwrap()).unwrap();
        state
    }

    /// Create folders from a list of folder names in a temp dir and initialize a test state in
    /// that dir. Note that the cursor position for the state will be 1, since the first item is '..'.
    fn create_test_state_with_folders(
        tmp: &TempDir,
        win_h: usize,
        folder_names: Vec<&str>,
    ) -> TereAppState {
        create_test_state_with_files_and_folders(tmp, win_h, vec![], folder_names)
    }

    /// Create a test state with 'n' folders named 'folder 1', 'folder 2', ... 'folder n'. Note
    /// that the ls_output_buf of the test state will contain n + 1 folders, since it will include
    /// the '..'.
    fn create_test_state_with_n_folders(
        tmp: &TempDir,
        win_h: usize,
        n_folders: usize,
    ) -> TereAppState {
        let fnames: Vec<_> = (1..=n_folders).map(|i| format!("folder {i}")).collect();
        create_test_state_with_folders(tmp, win_h, fnames.iter().map(|s| s.as_ref()).collect())
    }

    #[test]
    fn test_scrolling_bufsize_less_than_window_size() {
        //TODO: create a macro to hold onto tmp?
        let tmp = TempDir::new().unwrap();
        let mut state = create_test_state_with_n_folders(&tmp, 10, 4);

        for i in 2..=4 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, 4);
            assert_eq!(state.scroll_pos, 0);
        }

        state.move_cursor(100, false);
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.scroll_pos, 0);

        for i in 1..=4 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 4 - i);
            assert_eq!(state.scroll_pos, 0);
        }

        assert_eq!(state.cursor_pos, 0);

        for _ in 0..5 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, 0);
        }

        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        // test jumping all the way to the bottom and back
        state.move_cursor(100, false);
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }

    #[test]
    fn test_scrolling_bufsize_less_than_window_size_wrap() {
        let tmp = TempDir::new().unwrap();
        let mut state = create_test_state_with_n_folders(&tmp, 5, 4);

        state.move_cursor_to(0);
        for i in 0..4 {
            state.move_cursor(1, true);
            assert_eq!(state.cursor_pos, i + 1);
            assert_eq!(state.scroll_pos, 0);
        }
        // cursor should be at the bottom of the listing
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.scroll_pos, 0);

        // wrap around
        state.move_cursor(1, true);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        // wrap around backwards
        state.move_cursor(-1, true);
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.scroll_pos, 0);
    }

    #[test]
    fn test_scrolling_bufsize_equal_to_window_size() {
        let tmp = TempDir::new().unwrap();
        let mut state = create_test_state_with_n_folders(&tmp, 5, 4);

        for i in 2..=4 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, 4);
            assert_eq!(state.scroll_pos, 0);
        }

        for i in 1..=3 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 4 - i);
            assert_eq!(state.scroll_pos, 0);
        }

        // test jumping all the way to the bottom and back
        state.move_cursor(100, false);
        assert_eq!(state.cursor_pos, 4);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }

    fn test_scrolling_bufsize_larger_than_window_size_helper(win_h: usize, n_files: usize) {
        let tmp = TempDir::new().unwrap();
        let mut state = create_test_state_with_n_folders(&tmp, win_h, n_files);
        let max_cursor = win_h - 1;
        let max_scroll = n_files + 1 - win_h;

        state.move_cursor_to(0); // start from the top

        // move cursor all the way to the bottom of the window
        for i in 1..=max_cursor {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        // scroll to the end of the list
        for i in 1..=max_scroll {
            println!(
                "scrolling beyond screen {}, cursor at {}, scroll {}",
                i, state.cursor_pos, state.scroll_pos
            );
            state.move_cursor(1, false);
            println!(
                "after move: cursor at {}, scroll {}",
                state.cursor_pos, state.scroll_pos
            );
            assert_eq!(state.cursor_pos, max_cursor);
            assert_eq!(state.scroll_pos, i);
        }

        assert_eq!(state.scroll_pos, max_scroll);

        // check that nothing changes when trying to scroll further
        for _ in 0..5 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, max_cursor);
            assert_eq!(state.scroll_pos, max_scroll);
        }
        state.move_cursor(win_h as isize + 100, false);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the window
        for i in 1..=max_cursor {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, max_cursor - i);
            assert_eq!(state.scroll_pos, max_scroll);
        }
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the list
        for i in 1..=max_scroll {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, max_scroll - i);
        }

        // check that nothing changes when trying to scroll further
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
        for _ in 0..5 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, 0);
        }
        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        // test jumping all the way to the bottom and back
        state.move_cursor(win_h as isize + 100, false);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);
        state.move_cursor(-100 - win_h as isize, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size1() {
        test_scrolling_bufsize_larger_than_window_size_helper(4, 5);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size2() {
        test_scrolling_bufsize_larger_than_window_size_helper(4, 6);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size3() {
        test_scrolling_bufsize_larger_than_window_size_helper(4, 7);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size4() {
        test_scrolling_bufsize_larger_than_window_size_helper(4, 8);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size5() {
        test_scrolling_bufsize_larger_than_window_size_helper(4, 10);
    }

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size_wrap() {
        let tmp = TempDir::new().unwrap();
        let mut state = create_test_state_with_n_folders(&tmp, 4, 5);

        state.move_cursor_to(6);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 2);

        state.move_cursor(1, true);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        state.move_cursor(-1, true);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 2);
    }

    #[test]
    fn test_basic_advance_search() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 5, vec!["bar", "baz", "foo", "frob"]);
        s.move_cursor_to(2);

        // current state:
        //   ..
        //   bar
        // > baz
        //   foo
        //   frob

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");
        assert_eq!(s.cursor_pos, 3);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 4);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn test_advance_search_wrap() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 3, vec!["bar", "baz", "foo", "frob"]);
        s.move_cursor_to(4);

        // current state: ('|' shows the window position)
        //   ..
        //   bar
        //   baz   |
        //   foo   |
        // > frob  |

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 2);

        s.advance_search("b");

        // state should now be
        //   ..
        // > bar   |
        //   baz   |
        //   foo   |
        //   frob

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 1);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn test_advance_and_erase_search_with_cursor_on_match() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 6, vec!["bar", "baz", "foo", "frob"]);
        s.move_cursor_to(3);

        // current state:
        //   ..
        //   bar
        //   baz
        // > foo
        //   frob

        assert_eq!(s.cursor_pos, 3);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");

        // state shouldn't have changed

        assert_eq!(s.cursor_pos, 3);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 4);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 3);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 4);

        // we're now on 'baz'
        // erase the search char. should still stay at baz.
        s.erase_search_char();
        assert_eq!(s.cursor_pos, 4);
    }

    #[test]
    fn test_advance_and_erase_with_filter_search() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 6, vec!["bar", "baz", "foo", "frob"]);
        s._settings.filter_search = true;

        // current state:
        //   ..
        // > bar
        //   baz
        //   foo
        //   frob

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");

        // state should now be
        // > foo
        //   frob

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);
        assert_eq!(s.visible_items().len(), 2);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.erase_search_char();

        // now:
        //   ..
        //   bar
        //   baz
        // > foo
        //   frob

        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn test_advance_and_clear_with_filter_search() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 6, vec!["bar", "baz", "foo", "forb"]);
        s._settings.filter_search = true;

        // current state:
        //   ..
        // > bar
        //   baz
        //   foo
        //   forb

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");
        s.advance_search("o");

        // state should now be
        // > foo
        //   forb

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);
        let visible: Vec<_> = s
            .visible_items()
            .iter()
            .map(|x| x.file_name_checked())
            .collect();
        assert_eq!(visible, vec!["foo", "forb"]);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.clear_search();

        // now:
        //   ..
        //   bar
        //   baz
        // > foo
        //   forb

        assert_eq!(s.cursor_pos, 3);
        assert_eq!(s.visible_items().len(), s.ls_output_buf.all_items.len());
    }

    #[test]
    fn test_advance_search_with_filter_search_and_scrolling() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 3, vec!["foo", "frob", "bar", "baz"]);
        s._settings.filter_search = true;

        s.move_cursor_to(3);

        // current state: ('|' shows the window position)
        //   ..
        //   foo   |
        //   frob  |
        // > bar   |
        //   baz

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 1);

        s.advance_search("f");

        // state should now be
        // > foo   |
        //   frob  |
        //         |

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);
        assert_eq!(s.visible_items().len(), 2);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn test_advance_and_erase_search_with_filter_and_cursor_on_match() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 6, vec!["bar", "baz", "foo", "frob"]);
        s._settings.filter_search = true;
        s.move_cursor_to(2);

        // current state:
        //   ..
        //   bar
        // > baz
        //   foo
        //   frob

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");

        // state should now be
        //   bar
        // > baz

        let visible: Vec<_> = s
            .visible_items()
            .iter()
            .map(|x| x.file_name_checked())
            .collect();
        assert_eq!(visible, vec!["bar", "baz"]);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);

        // we're now at baz. erase char, we should still be at baz:
        //   ..
        //   bar
        // > baz
        //   foo
        //   frob

        s.erase_search_char();
        assert_eq!(s.cursor_pos, 2);

        let visible: Vec<_> = s
            .visible_items()
            .iter()
            .map(|x| x.file_name_checked())
            .collect();
        assert_eq!(visible, vec!["..", "bar", "baz", "foo", "frob"]);
    }

    #[test]
    fn test_advance_and_erase_search_with_filter_and_cursor_on_match2() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 6, vec!["bar", "baz", "foo", "frob"]);
        s._settings.filter_search = true;
        s.move_cursor_to(4);

        // current state:
        //   ..
        //   bar
        //   baz
        //   foo
        // > frob

        assert_eq!(s.cursor_pos, 4);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");

        // state should now be
        //   foo
        // > frob

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);

        // we're now on 'baz'
        // erase the search char. now the state should be
        //   ..
        //   foo
        //   frob
        //   bar
        // > baz

        s.erase_search_char();
        assert_eq!(s.cursor_pos, 4);
    }

    #[ignore]
    #[test]
    fn test_advance_and_erase_search_with_filter_and_cursor_on_match3() {
        // idea: the cursor should not move if it's not necessary

        let tmp = TempDir::new().unwrap();
        let mut s =
            create_test_state_with_folders(&tmp, 3, vec!["aaa", "baa", "bab", "bba", "caa", "cab"]);
        s._settings.filter_search = true;
        s.move_cursor_to(1);

        // current state:
        //   ..  |
        // > aaa |
        //   baa |
        //   bab
        //   bba
        //   caa
        //   cab

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");

        // current state:
        // > baa |
        //   bab |
        //   bba |

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);
        assert_eq!(s.visible_items().len(), 3);

        s.erase_search_char();

        // current state should be:
        //   ..
        //   aa
        // > baa |
        //   bab |
        //   bba |
        //   caa
        //   cab

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 2);
        assert_eq!(s.visible_items().len(), 7);
    }

    #[test]
    fn test_advance_and_erase_search_with_filter_and_scrolling() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 2, vec!["bar", "baz", "foo", "frob"]);
        s._settings.filter_search = true;

        // current state:
        //   ..   |
        // > bar  |
        //   baz
        //   foo
        //   frob

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");

        // state should now be
        // > foo
        //   frob

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);

        // we're now on 'bar'
        // erase the search char. now the state should be
        //   ..
        //   bar
        //   baz  |
        // > foo  |
        //   frob

        s.erase_search_char();
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 2);
    }

    #[test]
    fn test_advance_search_with_filter_search_and_scrolling2() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 3, vec!["bar", "baz", "foo", "frob"]);
        s._settings.filter_search = true;
        s.move_cursor_to(4);

        // current state:
        //   ..
        //   bar
        //   baz  |
        //   foo  |
        // > frob |

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 2);

        s.advance_search("f");

        // state should now be:
        //   foo  |
        // > frob |
        //        |

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);
    }

    #[test]
    fn test_search_default_file_handling_mode() {
        // by default, only folders should be searched
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_files_and_folders(&tmp, 3, vec!["foo"], vec!["frob"]);

        s.advance_search("f");
        assert_eq!(s.visible_match_indices(), vec![1]);
    }

    #[test]
    fn test_search_file_handling_mode_match() {
        // match both files & folders
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_files_and_folders(&tmp, 3, vec!["foo"], vec!["frob"]);

        s._settings.file_handling_mode = FileHandlingMode::Match;
        s.advance_search("f");
        assert_eq!(s.visible_match_indices(), vec![1, 2]);
    }

    #[test]
    fn test_search_file_handling_mode_hide() {
        // match only folders, and hide files
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_files_and_folders(&tmp, 3, vec!["foo"], vec!["frob"]);

        s._settings.file_handling_mode = FileHandlingMode::Hide;
        s.update_ls_output_buf().unwrap(); // to apply file handling mode
        assert_eq!(
            s.visible_items()
                .iter()
                .map(|e| e.file_name_checked())
                .collect::<Vec<_>>(),
            vec!["..", "frob"],
        );
    }

    #[test]
    fn test_filter_search_toggle() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["aaa", "aab", "bbb"]);
        s._settings.filter_search = false;
        s.cursor_pos = 1;

        // initial state:
        //   ..
        // > aaa
        //   aab
        //   bbb
        assert_eq!(s.cursor_pos, 1);

        s.advance_search("a");
        s.move_cursor_to_adjacent_match(1);

        // state should now be
        //   ..
        //   aaa
        // > aab
        //   bbb

        assert_eq!(s.scroll_pos, 0);
        assert_eq!(s.cursor_pos, 2);
        assert_eq!(
            s.visible_items()
                .iter()
                .map(|e| e.file_name_checked())
                .collect::<Vec<_>>(),
            vec!["..", "aaa", "aab", "bbb"]
        );

        // toggle the filter search
        s.set_filter_search(true);

        // now the state should be
        //   aaa
        // > aab

        assert_eq!(s.scroll_pos, 0);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(
            s.visible_items()
                .iter()
                .map(|e| e.file_name_checked())
                .collect::<Vec<_>>(),
            vec!["aaa", "aab"]
        );
    }

    #[test]
    fn test_case_sensitive_mode_change() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["A", "a"]);
        s.cursor_pos = 1;
        s.advance_search("a");

        // current state: ('*' shows the matches)
        //   ..
        // > A  *
        //   a  *

        assert_eq!(s.visible_match_indices(), vec![1, 2]);
        assert_eq!(s.cursor_pos, 1);

        s.set_case_sensitive(CaseSensitiveMode::CaseSensitive);
        assert_eq!(s.visible_match_indices(), vec![2]);
        assert_eq!(s.cursor_pos, 2);
    }

    #[test]
    fn test_gap_search_mode_change() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["aaa", "aab", "aba"]);
        s.cursor_pos = 1;
        s.advance_search("a");
        s.advance_search("b");

        // the state should now be
        //   ..
        //   aaa
        // > aab *
        //   aba *

        assert_eq!(s.visible_match_indices(), vec![2, 3]);
        assert_eq!(s.cursor_pos, 2);

        s.set_gap_search_mode(GapSearchMode::NormalSearch);

        // now it should be
        //   ...
        //   aaa
        //   aab
        // > aba *

        assert_eq!(s.visible_match_indices(), vec![3]);
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn test_cd_basic() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);
        assert_eq!(s.current_path, tmp.path());
        assert!(s.change_dir("foo").is_ok());
        assert_eq!(s.current_path, tmp.path().join("foo"));
    }

    #[test]
    fn test_cd_parent() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);
        assert_eq!(s.current_path, tmp.path());
        assert!(s.change_dir("..").is_ok());
        assert_eq!(s.current_path, tmp.path().parent().unwrap());
    }

    #[test]
    fn test_cd_item_under_cursor() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["bar", "foo"]);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.current_path, tmp.path());
        assert!(s.change_dir("").is_ok());
        assert_eq!(s.current_path, tmp.path().join("bar"));
        assert!(s.change_dir("..").is_ok());
        s.move_cursor(1, false);
        assert_eq!(s.cursor_pos, 2);
        assert!(s.change_dir("").is_ok());
        assert_eq!(s.current_path, tmp.path().join("foo"));
    }

    #[test]
    fn test_cd_nonexistent() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);
        assert_eq!(s.current_path, tmp.path());
        let res = s.change_dir("bar");
        match res {
            Ok(CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            }) => {
                assert_eq!(p, tmp.path().join("bar"));
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            something_else => panic!("{:?}", something_else),
        }
        assert_eq!(s.current_path, tmp.path());
    }

    #[test]
    fn test_cd_root() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);
        assert_eq!(s.current_path, tmp.path());
        assert!(s.change_dir("/").is_ok());
        assert_eq!(s.current_path, PathBuf::from("/"));
    }

    #[test]
    fn test_find_valid_cd_target() {
        let tmp = TempDir::new().unwrap();
        let s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);

        // valid target
        let (path, res) = s.find_valid_cd_target(&PathBuf::from("foo")).unwrap();
        assert_eq!(path, tmp.path().join("foo"));
        match res {
            CdResult::Success => {}
            something_else => panic!("{:?}", something_else),
        }

        // target not found
        let (path, res) = s.find_valid_cd_target(&PathBuf::from("invalid")).unwrap();
        assert_eq!(path, tmp.path());
        match res {
            CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            } => {
                assert_eq!(p, tmp.path().join("invalid"));
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            something_else => panic!("{:?}", something_else),
        }

        // root
        let (path, res) = s.find_valid_cd_target(&PathBuf::from("/")).unwrap();
        assert_eq!(path, PathBuf::from("/"));
        match res {
            CdResult::Success => {}
            something_else => panic!("{:?}", something_else),
        }

        // valid target is root
        let (path, res) = s.find_valid_cd_target(&PathBuf::from("/foo/bar")).unwrap();
        assert_eq!(path, PathBuf::from("/"));
        match res {
            CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            } => {
                assert_eq!(p, PathBuf::from("/foo/bar"));
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            something_else => panic!("{:?}", something_else),
        }
    }

    #[test]
    fn test_cd_current_dir_deleted() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);

        s.change_dir("foo").unwrap();
        let path = tmp.path().join("foo");
        std::fs::remove_dir(&path).unwrap();
        let res = s.change_dir(".").unwrap();

        assert_eq!(s.current_path, tmp.path());
        match res {
            CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            } => {
                assert_eq!(p, path);
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            something_else => panic!("{:?}", something_else),
        }
    }

    #[test]
    fn test_cd_current_dir_parent_deleted() {
        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);
        let target_path = tmp.path().join("foo").join("bar");
        std::fs::create_dir(&target_path).unwrap();

        s.change_dir("foo").unwrap();
        s.change_dir("bar").unwrap();
        std::fs::remove_dir_all(tmp.path().join("foo")).unwrap();

        let res = s.change_dir(".").unwrap();

        assert_eq!(s.current_path, tmp.path());
        match res {
            CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            } => {
                assert_eq!(p, target_path);
                assert_eq!(e.kind(), ErrorKind::NotFound);
            }
            something_else => panic!("{:?}", something_else),
        }
    }

    #[test]
    #[cfg(unix)] // permissions can only be changed on unix (see PermissionsExt) as of 2023 July
    fn test_cd_current_dir_permission_denied() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = TempDir::new().unwrap();
        let mut s = create_test_state_with_folders(&tmp, 10, vec!["foo"]);

        s.change_dir("foo").unwrap();
        assert_eq!(s.current_path, tmp.path().join("foo"));

        let path = tmp.path().join("foo");
        let original_perms = path.metadata().unwrap().permissions();

        // set perms to write-only
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o222)).unwrap();

        let res = s.change_dir(".").unwrap();

        assert_eq!(s.current_path, tmp.path());
        match res {
            CdResult::MovedUpwards {
                target_abs_path: p,
                root_error: e,
            } => {
                assert_eq!(p, path);
                assert_eq!(e.kind(), ErrorKind::PermissionDenied);
            }
            something_else => panic!("{:?}", something_else),
        }

        // set permissions back to original so that tempfile can delete it
        std::fs::set_permissions(&path, original_perms).unwrap();
    }
}
