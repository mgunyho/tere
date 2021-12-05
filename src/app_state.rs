/// This module contains structs related to handling the application state,
/// independent of a "graphical" front-end, such as `ncurses`.

use clap::ArgMatches;

use std::convert::TryFrom;
use std::ffi::OsStr;

#[path = "settings.rs"]
mod settings;
use settings::TereSettings;
pub use settings::CaseSensitiveMode;

#[path = "history.rs"]
mod history;
use history::HistoryTreeEntry;

pub const NO_MATCHES_MSG: &str = "No matches";

/// A vector that keeps track of items that are 'filtered'. It offers indexing/viewing
/// both the vector of filtered items and the whole unfiltered vector.
struct FilteredVec<T> {
    all_items: Vec<T>,
    // This vec contains the indices of the items that have not been filtered out
    kept_indices: Vec<usize>,
}

impl<T> FilteredVec<T> {

    /// Return a vector of all items that have been kept
    pub fn kept_items(&self) -> Vec<&T> {
        self.kept_indices.iter().filter_map(|idx| self.all_items.get(*idx))
            .collect()
    }

    /// Recreate the collection of filtered items by through all items in the unfiltered collection
    /// and applying a filter function
    pub fn apply_filter<F>(&mut self, filter: F)
    where
        F: Fn(&T) -> bool
    {
        self.kept_indices.clear();
        self.kept_indices = self.all_items.iter()
            .enumerate()
            .filter(|(_, x)| filter(&x))
            .map(|(i, _)| i)
            .collect();
    }

    /// Clear the filtered results, so that the kept items contain all items
    pub fn clear_filter(&mut self) {
        self.kept_indices.clear();
        self.kept_indices = (0..self.all_items.len()).collect();
    }
}

impl<T> From<Vec<T>> for FilteredVec<T> {
    fn from(vec: Vec<T>) -> Self {
        let mut ret = Self {
            all_items: vec,
            kept_indices: vec![],
        };
        ret.clear_filter();
        ret
    }
}


/// A stripped-down version of ``std::fs::DirEntry``.
#[derive(Clone)]
pub struct CustomDirEntry {
    _path: std::path::PathBuf,
    pub metadata: Option<std::fs::Metadata>,
    _file_name: std::ffi::OsString,
}

impl CustomDirEntry {
    /// Return the file name of this directory entry. The file name is an OsString,
    /// which may not be possible to convert to a String. In this case, this
    /// function returns an empty string.
    pub fn file_name_checked(&self) -> String {
        self._file_name.clone().into_string().unwrap_or("".to_string())
    }
    pub fn path(&self) -> &std::path::PathBuf { &self._path }
    pub fn is_dir(&self) -> bool {
        match &self.metadata {
            Some(m) => m.is_dir(),
            None => false,
        }
    }
}

impl From<std::fs::DirEntry> for CustomDirEntry
{
    fn from(e: std::fs::DirEntry) -> Self {
        Self {
            _path: e.path(),
            metadata: std::fs::metadata(e.path()).ok(),
            _file_name: e.file_name(),
        }
    }
}

impl From<&std::path::Path> for CustomDirEntry
{
    fn from(p: &std::path::Path) -> Self {
        Self {
            _path: p.to_path_buf(),
            metadata: p.metadata().ok(),
            _file_name: p.file_name().unwrap_or(p.as_os_str()).to_os_string(),
        }
    }
}


type LsBufItem = CustomDirEntry;
/// The type of the `ls_output_buf` buffer of the app state
type LsBufType = FilteredVec<LsBufItem>;


/// This struct represents the state of the application. Note that it has no
/// notion of curses windows.
pub struct TereAppState {

    // Width and height of the main window. These values have to be updated by
    // calling using the update_main_window_dimensions function if the window
    // dimensions change.
    main_win_w: u32,
    main_win_h: u32,

    // This vector will hold the list of files/folders in the current directory,
    // including ".." (the parent folder).
    ls_output_buf: LsBufType,

    //sort_mode: SortMode // TODO: sort by date etc

    // The row on which the cursor is currently on, counted starting from the
    // top of the screen (not from the start of `ls_output_buf`). Note that this
    // doesn't have anything to do with the crossterm cursor position.
    pub cursor_pos: u32,

    // The top of the screen corresponds to this row in the `ls_output_buf`.
    pub scroll_pos: u32,

    search_string: String,

    pub header_msg: String,
    pub info_msg: String,

    pub settings: TereSettings,
}

impl TereAppState {
    pub fn init(cli_args: &ArgMatches, window_w: u32, window_h: u32) -> Result<Self, clap::Error> {
        let mut ret = Self {
            main_win_w: window_w,
            main_win_h: window_h,
            ls_output_buf: vec![].into(),
            cursor_pos: 0, // TODO: get last value from previous run
            scroll_pos: 0,
            header_msg: "".into(),
            info_msg: "".into(), // TODO: initial help message, like 'tere vXXX, type "?" for help'
            search_string: "".into(),
            //search_anywhere: false,
            settings: TereSettings::parse_cli_args(cli_args)?,
        };

        ret.update_header();
        ret.update_ls_output_buf();
        Ok(ret)
    }

    ///////////////////////////////////////////
    // Helpers for reading the current state //
    ///////////////////////////////////////////

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
        self.ls_output_buf.kept_indices.len()
    }

    /// Return a vector that contains the indices into the currently visible
    /// items that contain a match
    pub fn visible_match_indices(&self) -> Vec<usize> {
        if self.settings.filter_search {
            (0..self.ls_output_buf.kept_indices.len()).collect()
        } else {
            // it's ok to clone here, the kept_indices will be usually quite short.
            self.ls_output_buf.kept_indices.clone()
        }
    }

    pub fn visible_items(&self) -> Vec<&LsBufItem> {
        if self.settings.filter_search {
            self.ls_output_buf.kept_items()
        } else {
            self.ls_output_buf.all_items.iter().collect()
        }
    }

    /// Convert a cursor position (in the range 0..window_height) to an index
    /// into the currently visible items.
    pub fn cursor_pos_to_visible_item_index(&self, cursor_pos: u32) -> usize {
        (cursor_pos + self.scroll_pos) as usize
    }

    pub fn get_item_at_cursor_pos(&self, cursor_pos: u32) -> Option<&LsBufItem> {
        let idx = self.cursor_pos_to_visible_item_index(cursor_pos) as usize;
        self.visible_items().get(idx).map(|x| *x)
    }

    /// Returns None if the visible items is empty, or if the state is
    /// inconsistent and the cursor is outside the currently visible items.
    fn get_item_under_cursor(&self) -> Option<&LsBufItem> {
        self.get_item_at_cursor_pos(self.cursor_pos)
    }

    /// Get the index of a filename into the currently visible items. Returns
    /// None if it's not found.
    fn index_of_filename<S: AsRef<OsStr>>(&self, fname: S) -> Option<usize> {
        self.visible_items().iter()
            .position(|x| {
                AsRef::<OsStr>::as_ref(&x.file_name_checked()) == fname.as_ref()
            })
    }

    //////////////////////////////////////
    // Functions for updating the state //
    //////////////////////////////////////

    pub fn update_header(&mut self) {
        //TODO: add another row to header (or footer?) with info, like 'tere - type ALT+? for help', and show status message when trying to open file etc
        let cwd: std::string::String = match std::env::current_dir() {
            Ok(path) => format!("{}", path.display()),
            Err(e) => format!("Unable to get current dir! ({})", e),
        };
        self.header_msg = cwd;
    }

    pub fn update_main_window_dimensions(&mut self, w: u32, h: u32) {
        let delta_h = h.checked_sub(self.main_win_h).unwrap_or(0);
        self.main_win_w = w;
        self.main_win_h = h;
        self.move_cursor(0, false); // make sure that cursor is within view
        if delta_h > 0 {
            // height is increasing, scroll backwards as much as possible
            let old_scroll_pos = self.scroll_pos;
            self.scroll_pos = self.scroll_pos.checked_sub(delta_h).unwrap_or(0);
            self.cursor_pos += old_scroll_pos - self.scroll_pos;
        }
    }

    pub fn update_ls_output_buf(&mut self) {
        if let Ok(entries) = std::fs::read_dir(".") {
            let pardir = std::path::Path::new(&std::path::Component::ParentDir);
            let mut new_output_buf: Vec<LsBufItem> = vec![CustomDirEntry::from(pardir).into()].into();

            let mut entries: Box<dyn Iterator<Item = LsBufItem>> =
                Box::new(
                //TODO: sort by date etc... - collect into vector of PathBuf's instead of strings (check out `Pathbuf::metadata()`)
                //TODO: case-insensitive sort???
                //TODO: cache file metadata already here when reloading it
                entries.filter_map(|e| e.ok()).map(|e| CustomDirEntry::from(e).into())
                );

            if self.settings.folders_only {
                entries = Box::new(entries.filter(|e| e.path().is_dir()));
            }

            new_output_buf.extend(
                entries
            );

            new_output_buf.sort_by(|a, b| {
                //NOTE: partial_cmp for strings always returns Some, so unwrap is ok here
                //a.file_name_checked().partial_cmp(&b.file_name_checked()).unwrap()
                match (a.is_dir(), b.is_dir()) {
                    (true, true) | (false, false) => {
                        // both are dirs or files, compare by name
                        a.file_name_checked().partial_cmp(&b.file_name_checked()).unwrap()
                    },
                    // Otherwise, put folders first
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                }
            });

            self.ls_output_buf = new_output_buf.into();
        }
        //TODO: show error message (add separate msg box)
    }

    pub fn change_dir(&mut self, path: &str) -> std::io::Result<()> {
        // TODO: add option to use xdg-open (or similar) on files?
        // check out https://crates.io/crates/open
        // (or https://docs.rs/opener/0.4.1/opener/)
        let final_path = if path.is_empty() {
            //TODO: error here if result is empty?
            self.get_item_under_cursor()
                .map_or("".to_string(), |s| s.file_name_checked())
        } else {
            path.to_string()
        };
        let old_cwd = std::env::current_dir();
        self.clear_search();
        std::env::set_current_dir(&final_path)?;
        self.update_ls_output_buf();
        //TODO: proper history
        self.cursor_pos = 0;
        self.scroll_pos = 0;
        if let Ok(old_cwd) = old_cwd {
            if let Some(old_cwd) = old_cwd.file_name() {
                if let Some(idx) = self.index_of_filename(old_cwd) {
                    self.move_cursor(idx as i32, false);
                }
            }
        }
        Ok(())
    }

    /////////////////////////////////////
    // Functions for moving the cursor //
    /////////////////////////////////////

    /// Move the cursor up (positive amount) or down (negative amount) in the
    /// currently visible items, and update the scroll position as necessary
    pub fn move_cursor(&mut self, amount: i32, wrap: bool) {
        //TOOD: wrap around (when starting from the last row)

        let old_cursor_pos = self.cursor_pos;
        let old_scroll_pos = self.scroll_pos;
        let visible_items = self.visible_items();
        let n_visible_items = visible_items.len() as u32;
        let max_y = self.main_win_h;

        let mut new_cursor_pos: i32 = old_cursor_pos as i32 + amount;

        if wrap && !visible_items.is_empty() {
            let offset = self.scroll_pos as i32;
            new_cursor_pos = (offset + new_cursor_pos)
                .rem_euclid(n_visible_items as i32) - offset;
        }

        if new_cursor_pos < 0 {
            // attempting to go above the current view, scroll up
            self.scroll_pos = self.scroll_pos
                .checked_sub(new_cursor_pos.abs() as u32).unwrap_or(0);
            self.cursor_pos = 0;
        } else if new_cursor_pos as u32 + old_scroll_pos >= n_visible_items {
            // attempting to go below content
            //TODO: wrap, but only if cursor is starting off at the last row
            // i.e. if pressing pgdown before the end, jump only to the end,
            // but if pressing pgdown at the very end, wrap and start from top
            self.scroll_pos = n_visible_items.checked_sub(max_y).unwrap_or(0);
            self.cursor_pos = n_visible_items.checked_sub(self.scroll_pos + 1)
                .unwrap_or(0);
        } else if new_cursor_pos as u32 >= max_y {
            // Attempting to go below current view, scroll down.
            self.cursor_pos = max_y - 1;
            self.scroll_pos = std::cmp::min(
                n_visible_items,
                old_scroll_pos + new_cursor_pos as u32
            ).checked_sub(self.cursor_pos).unwrap_or(0);
        } else {
            // scrolling within view
            self.cursor_pos = new_cursor_pos as u32;
        }

    }

    /// Move the cursor so that it is at the location `row` in the
    /// currently visible items, and update the scroll position as necessary
    pub fn move_cursor_to(&mut self, row: u32) {
        self.move_cursor(row as i32
                         - self.cursor_pos as i32
                         - self.scroll_pos as i32,
                         false);
    }

    /// Move cursor to the position of a given filename. If the filename was
    /// not found, don't move the cursor and return false, otherwise return true.
    pub fn move_cursor_to_filename<S: AsRef<OsStr>>(&mut self, fname: S) -> bool {
        self.index_of_filename(fname)
            .map(|idx| self.move_cursor_to(u32::try_from(idx).unwrap_or(u32::MAX)))
            .is_some()
    }


    /// Move the cursor to the next or previous match in the current list of
    /// matches. If dir is positive, move to the next match, if it's negative,
    /// move to the previous match, and if it's zero, move to the cursor to the
    /// current match.
    pub fn move_cursor_to_adjacent_match(&mut self, dir: i32) {
        if self.num_matching_items() > 0 && self.is_searching() {

            if self.settings.filter_search {
                // the only visible items are the matches, so we can just move the cursor
                self.move_cursor(dir.signum(), true);
            } else {

                let cur_idx = self.cursor_pos_to_visible_item_index(self.cursor_pos);
                let kept_indices = &self.ls_output_buf.kept_indices;
                let (cur_idx_in_kept, cur_idx_in_all) = kept_indices.iter()
                    .enumerate()
                    .skip_while(|(_, i_in_all)| **i_in_all < cur_idx)
                    .next()
                    // if we skipped everything, wrap around and return the first
                    // item in the kept indices. shouldn't panic, kept_indices
                    // shouldn't be empty based on the visible_items().len()
                    // check above.
                    .unwrap_or((0, &kept_indices[0]));

                let i = if dir < 0 {
                    let i = cur_idx_in_kept.checked_sub(1).unwrap_or(kept_indices.len() - 1);
                    kept_indices[i]
                } else if dir > 0 {
                    let i = (cur_idx_in_kept + 1) % kept_indices.len();
                    kept_indices[i]
                } else {
                    // dir == 0, just use the current index
                    *cur_idx_in_all
                };

                self.move_cursor_to(u32::try_from(i).unwrap_or(u32::MAX));
            }
        }
    }

    ///////////
    // Seach //
    ///////////

    fn update_search_matches(&mut self) {
        let is_case_sensitive = match self.settings.case_sensitive {
            CaseSensitiveMode::IgnoreCase => false,
            CaseSensitiveMode::CaseSensitive => true,
            CaseSensitiveMode::SmartCase => {
                self.search_string.chars().any(|c| c.is_uppercase())
            }
        };
        let search_string = if is_case_sensitive {
            self.search_string.clone()
        } else {
            self.search_string.to_lowercase()
        };
        self.ls_output_buf.apply_filter(|itm| {
            let target = if is_case_sensitive {
                itm.file_name_checked()
            } else {
                itm.file_name_checked().to_lowercase()
            };
            target.starts_with(&search_string)
        });
    }

    pub fn clear_search(&mut self) {
        let previous_item_under_cursor = self.get_item_under_cursor().cloned();
        self.search_string.clear();
        self.ls_output_buf.clear_filter();
        previous_item_under_cursor.map(|itm| self.move_cursor_to_filename(itm.file_name_checked()));
    }

    pub fn advance_search(&mut self, query: &str) {
        self.search_string.push_str(query);

        let previous_item_under_cursor = self.get_item_under_cursor().cloned();

        self.update_search_matches();

        if self.settings.filter_search {
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
        if let Some(_) = self.search_string.pop() {
            //TODO: keep cursor position when there were no matches? should somehow push cursor position onto some stack when advancing search.

            let previous_item_under_cursor = self.get_item_under_cursor().cloned();

            self.update_search_matches();

            if self.settings.filter_search {
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

#[cfg(test)]
mod tests_for_filtered_vec {
    use super::FilteredVec;

    #[test]
    fn test_filter_basic() {
        let mut v = FilteredVec::from(vec![1, 2, 3]);
        v.apply_filter(|n| (n % 2) == 0);
        assert_eq!(v.all_items, vec![1, 2, 3]);
        assert_eq!(v.kept_items(), vec![&2]);
        assert_eq!(v.kept_indices, vec![1]);
    }

    #[test]
    fn test_clear_filter() {
        let mut v = FilteredVec::from(vec![1, 2, 3]);
        v.apply_filter(|_| false);
        assert_eq!(v.kept_items(), Vec::<&usize>::new());
        v.clear_filter();
        assert_eq!(v.kept_items(), vec![&1, &2, &3]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_filenames(n: u32) -> LsBufType {
        let fnames: Vec<_> = (1..=n).map(|i| format!("file {}", i)).collect();
        strings_to_ls_buf(fnames)
    }

    fn strings_to_ls_buf<S: AsRef<std::ffi::OsStr>>(strings: Vec<S>) -> LsBufType {
        strings.iter()
            .map(|s| CustomDirEntry::from(std::path::PathBuf::from(&s).as_ref()))
            .collect::<Vec<CustomDirEntry>>()
            .into()
    }

    fn create_test_state(win_h: u32, n_filenames: u32) -> TereAppState {
        create_test_state_with_buf(win_h, create_test_filenames(n_filenames))
    }

    fn create_test_state_with_buf(win_h: u32,
                                  buf: LsBufType) -> TereAppState {
        TereAppState {
            cursor_pos: 0,
            scroll_pos: 0,
            main_win_h: win_h,
            main_win_w: 10,
            ls_output_buf: buf,
            header_msg: "".into(),
            info_msg: "".into(),
            search_string: "".into(),
            settings: Default::default(),
        }
    }

    #[test]
    fn test_scrolling_bufsize_less_than_window_size() {
        let mut state = create_test_state(10, 4);

        for i in 1..=3 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, 0);
        }

        state.move_cursor(100, false);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);

        for i in 1..=3 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 3 - i);
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
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }

    #[test]
    fn test_scrolling_bufsize_equal_to_window_size() {
        let mut state = create_test_state(4, 4);

        for i in 1..=3 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, 0);
        }

        for i in 1..=3 {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 3-i);
            assert_eq!(state.scroll_pos, 0);
        }

        // test jumping all the way to the bottom and back
        state.move_cursor(100, false);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100, false);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

    }

    //TODO: use rstest? https://stackoverflow.com/a/52843365
    // (using dev-dependencies, https://doc.rust-lang.org/rust-by-example/testing/dev_dependencies.html)
    fn test_scrolling_bufsize_larger_than_window_size_helper(win_h: u32,
                                                             n_files: u32) {
        let mut state = create_test_state(win_h, n_files);
        let max_cursor = win_h - 1;
        let max_scroll = n_files - win_h;

        // move cursor all the way to the bottom of the window
        for i in 1..=max_cursor {
            state.move_cursor(1, false);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        // scroll to the end of the list
        for i in 1..=max_scroll {
            println!("scrolling beyond screen {}, cursor at {}, scroll {}",
                     i, state.cursor_pos, state.scroll_pos);
            state.move_cursor(1, false);
            println!("after move: cursor at {}, scroll {}",
                     state.cursor_pos, state.scroll_pos);
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
        state.move_cursor(win_h as i32 + 100, false);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the window
        for i in 1..=max_cursor {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, max_cursor-i);
            assert_eq!(state.scroll_pos, max_scroll);
        }
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the list
        for i in 1..=max_scroll {
            state.move_cursor(-1, false);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, max_scroll-i);
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
        state.move_cursor(win_h as i32 + 100, false);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);
        state.move_cursor(-100 - win_h as i32, false);
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
    fn test_basic_advance_search() {
        let mut s = create_test_state_with_buf(5, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.move_cursor_to(2);

        // current state:
        //   ..
        //   foo
        // > frob
        //   bar
        //   baz

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");
        assert_eq!(s.cursor_pos, 3);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 4);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 3);
    }

    #[test]
    fn test_advance_search_wrap() {
        let mut s = create_test_state_with_buf(3, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.move_cursor_to(4);

        // current state: ('|' shows the window position)
        //   ..
        //   foo
        //   frob  |
        //   bar   |
        // > baz   |

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 2);

        s.advance_search("f");

        // state should now be
        //   ..
        // > foo   |
        //   frob  |
        //   bar   |
        //   baz

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 1);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
    }

    #[test]
    fn test_advance_and_erase_search_with_cursor_on_match() {
        let mut s = create_test_state_with_buf(6, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.move_cursor_to(3);

        // current state:
        //   ..
        //   foo
        //   frob
        // > bar
        //   baz

        assert_eq!(s.cursor_pos, 3);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");

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
        let mut s = create_test_state_with_buf(6, strings_to_ls_buf(
            vec![
                "..",
                "bar",
                "baz",
                "foo",
                "frob",
            ])
        );
        s.settings.filter_search = true;

        // current state:
        // > ..
        //   bar
        //   baz
        //   foo
        //   frob

        assert_eq!(s.cursor_pos, 0);
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
        let mut s = create_test_state_with_buf(6, strings_to_ls_buf(
            vec![
                "..",
                "bar",
                "baz",
                "foo",
                "forb",
            ])
        );
        s.settings.filter_search = true;

        // current state:
        // > ..
        //   bar
        //   baz
        //   foo
        //   forb

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");
        s.advance_search("o");

        // state should now be
        // > foo
        //   forb

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);
        let visible: Vec<_> = s.visible_items().iter().map(|x| x.file_name_checked()).collect();
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
        let mut s = create_test_state_with_buf(3, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.settings.filter_search = true;

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
        let mut s = create_test_state_with_buf(6, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.settings.filter_search = true;
        s.move_cursor_to(2);

        // current state:
        //   ..
        //   foo
        // > frob
        //   bar
        //   baz

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("f");

        // state should now be
        //   foo
        // > frob

        let visible: Vec<_> = s.visible_items().iter().map(|x| x.file_name_checked()).collect();
        assert_eq!(visible, vec!["foo", "frob"]);
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);

        // we're now at frob. erase char, we should still be at frob:
        //   ..
        //   foo
        // > frob
        //   bar
        //   baz

        s.erase_search_char();
        assert_eq!(s.cursor_pos, 2);

        let visible: Vec<_> = s.visible_items().iter().map(|x| x.file_name_checked()).collect();
        assert_eq!(visible, vec!["..", "foo", "frob", "bar", "baz"]);

    }

    #[test]
    fn test_advance_and_erase_search_with_filter_and_cursor_on_match2() {
        let mut s = create_test_state_with_buf(6, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.settings.filter_search = true;
        s.move_cursor_to(4);

        // current state:
        //   ..
        //   foo
        //   frob
        //   bar
        // > baz

        assert_eq!(s.cursor_pos, 4);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");

        // state should now be
        //   bar
        // > baz

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

    #[test]
    fn test_advance_and_erase_search_with_filter_and_scrolling() {
        let mut s = create_test_state_with_buf(2, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.settings.filter_search = true;

        // current state:
        // > ..   |
        //   foo  |
        //   frob
        //   bar
        //   baz

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.advance_search("b");

        // state should now be
        // > bar
        //   baz

        assert_eq!(s.cursor_pos, 0);
        assert_eq!(s.scroll_pos, 0);

        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 1);
        s.move_cursor_to_adjacent_match(1);
        assert_eq!(s.cursor_pos, 0);

        // we're now on 'bar'
        // erase the search char. now the state should be
        //   ..
        //   foo
        //   frob |
        // > bar  |
        //   baz

        s.erase_search_char();
        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 2);

    }

    #[test]
    fn test_advance_search_with_filter_search_and_scrolling2() {
        let mut s = create_test_state_with_buf(3, strings_to_ls_buf(
            vec![
                "..",
                "foo",
                "frob",
                "bar",
                "baz",
            ])
        );
        s.settings.filter_search = true;
        s.move_cursor_to(4);

        // current state:
        //   ..
        //   foo
        //   frob |
        //   bar  |
        // > baz  |

        assert_eq!(s.cursor_pos, 2);
        assert_eq!(s.scroll_pos, 2);

        s.advance_search("b");

        // state should now be:
        //   bar  |
        // > baz  |
        //        |

        assert_eq!(s.cursor_pos, 1);
        assert_eq!(s.scroll_pos, 0);
    }

}
