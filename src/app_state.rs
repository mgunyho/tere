/// This module contains structs related to handling the application state,
/// independent of a "graphical" front-end, such as `ncurses`.

/// A vector containing a list of matches, which also keeps track of which element
/// we're pointing at currently
#[derive(Default)]
pub struct MatchesVec<T> {
    vec: Vec<T>,
    idx: usize,
}

impl<T> MatchesVec<T> {
    pub fn increment(&mut self) {
        self.idx = (self.idx + 1) % self.vec.len();
    }

    pub fn decrement(&mut self) {
        self.idx = self.idx.checked_sub(1).unwrap_or(self.vec.len()-1);
    }

    /// The match we're currently pointing at. Returns None if the list of matches
    /// is empty.
    pub fn current_item(&self) -> Option<&T> {
        self.vec.get(self.idx)
    }

    /// The index of the match we're currently pointing at. Returs None if the
    /// list of matches is empty.
    pub fn current_pos(&self) -> Option<usize> {
        if self.vec.is_empty() {
            None
        } else {
            Some(self.idx)
        }
    }

    pub fn clear(&mut self) {
        self.vec.clear();
        self.idx = 0;
    }

    pub fn len(&self) -> usize {
        self.vec.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.vec.iter()
    }
}

impl<T> std::iter::FromIterator<T> for MatchesVec<T> {
    fn from_iter<I: std::iter::IntoIterator<Item=T>>(iter: I) -> Self {
        Self {
            vec: iter.into_iter().collect(),
            idx: 0,
        }
    }
}

type MatchType = (usize, String);
type MatchesType = MatchesVec<MatchType>;

#[derive(Default)]
struct SearchState {
    search_string: String,
    // This deque contains the items in `ls_output_buf` that match the current
    // search string, along with their indices (so it's possible to figure out
    // whether they are in view). The first element in this vector is always
    // the current match we're looking at, and it will be rotated around
    // (using rotate_left/right) when seeking the matches.
    matches: MatchesType,
}

impl SearchState {
    /// Reset the search state
    fn clear(&mut self) {
        self.matches.clear();
        self.search_string.clear();
    }

    /// Find the elements in `buf` that match with the current
    /// search string and store them in self.matches.
    fn update_matches(&mut self, buf: &Vec<String>) {
        // TODO: if matches is not empty, iterate over only that and not the whole buffer?
        self.matches = buf.iter().enumerate().filter(|(_, s)|
            //TODO: take search_anywhere into account
            //TODO: case sensitivity
            s.starts_with(&self.search_string)
        ).map(|(i, s)| (i, s.clone())).collect();
        //TODO: change indices -> Option<usize>, and put Some only for those that are within view?
    }
}

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
    pub ls_output_buf: Vec<String>,

    // The row on which the cursor is currently on, counted starting from the
    // top of the screen (not from the start of `ls_output_buf`). Note that this
    // doesn't have anything to do with the ncurses curspor position.
    pub cursor_pos: u32,

    // The top of the screen corresponds to this row in the `ls_output_buf`.
    pub scroll_pos: u32,

    //TODO
    //// if this is true, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool,

    search_state: SearchState,

    pub header_msg: String,
    pub info_msg: String,
}

impl TereAppState {
    pub fn init(window_w: u32, window_h: u32) -> Self {
        let mut ret = Self {
            main_win_w: window_w,
            main_win_h: window_h,
            ls_output_buf: vec![],
            cursor_pos: 0, // TODO: get last value from previous run
            scroll_pos: 0,
            header_msg: "".into(),
            info_msg: "".into(), // TODO: initial help message, like 'tere vXXX, type "?" for help'
            search_state: Default::default(),
            //search_anywhere: false,
        };

        ret.update_header();
        ret.update_ls_output_buf();
        return ret;
    }

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
        self.move_cursor(0); // make sure that cursor is within view
        if delta_h > 0 {
            // height is increasing, scroll backwards as much as possible
            let old_scroll_pos = self.scroll_pos;
            self.scroll_pos = self.scroll_pos.checked_sub(delta_h).unwrap_or(0);
            self.cursor_pos += old_scroll_pos - self.scroll_pos;
        }
    }

    pub fn update_ls_output_buf(&mut self) {
        if let Ok(entries) = std::fs::read_dir(".") {
            self.ls_output_buf = vec!["..".into()];
            self.ls_output_buf.extend(
                //TODO: sort by date etc... - collect into vector of PathBuf's instead of strings (check out `Pathbuf::metadata()`)
                //TODO: case-insensitive sort???
                //TODO: config option: show only folders, hide files
                entries.filter_map(|e| e.ok())
                    .map(|e| e.file_name().into_string().ok())
                    .filter_map(|e| e)
            );
            self.ls_output_buf.sort();
        }
        //TODO: show error message (add separate msg box)
    }

    /// Move the cursor up (positive amount) or down (negative amount), and scroll
    /// the view as necessary
    pub fn move_cursor(&mut self, amount: i32) {
        //TOOD: wrap around (when starting from the last row)

        let old_cursor_pos = self.cursor_pos;
        let old_scroll_pos = self.scroll_pos;
        let ls_buf_size = self.ls_output_buf.len() as u32;
        let max_y = self.main_win_h;

        let new_cursor_pos: i32 = old_cursor_pos as i32 + amount;

        if new_cursor_pos < 0 {
            // attempting to go above the current view, scroll up
            self.scroll_pos = self.scroll_pos
                .checked_sub(new_cursor_pos.abs() as u32).unwrap_or(0);
            self.cursor_pos = 0;
        } else if new_cursor_pos as u32 + old_scroll_pos >= ls_buf_size {
            // attempting to go below content
            //TODO: wrap, but only if cursor is starting off at the last row
            // i.e. if pressing pgdown before the end, jump only to the end,
            // but if pressing pgdown at the very end, wrap and start from top
            self.scroll_pos = ls_buf_size.checked_sub(max_y).unwrap_or(0);
            self.cursor_pos = ls_buf_size.checked_sub(self.scroll_pos + 1)
                .unwrap_or(0);
        } else if new_cursor_pos as u32 >= max_y {
            // Attempting to go below current view, scroll down.
            self.cursor_pos = max_y - 1;
            self.scroll_pos = std::cmp::min(
                ls_buf_size,
                old_scroll_pos + new_cursor_pos as u32
            ).checked_sub(self.cursor_pos).unwrap_or(0);
        } else {
            // scrolling within view
            self.cursor_pos = new_cursor_pos as u32;
        }

    }

    /// Move the cursor so that it is at the location `row` in the
    /// `ls_output_buf`, and scroll the view as necessary
    pub fn move_cursor_to(&mut self, row: u32) {
        self.move_cursor(row as i32
                         - self.cursor_pos as i32
                         - self.scroll_pos as i32);
    }

    pub fn change_dir(&mut self, path: &str) -> std::io::Result<()> {
        let final_path: &str = if path.is_empty() {
            let idx = self.cursor_pos + self.scroll_pos;
            self.ls_output_buf.get(idx as usize).map(|s| s.as_ref())
                .unwrap_or("")
        } else {
            path
        };
        let old_cwd = std::env::current_dir();
        self.search_state.clear();
        std::env::set_current_dir(final_path)?;
        self.update_ls_output_buf();
        //TODO: proper history
        self.cursor_pos = 0;
        self.scroll_pos = 0;
        if let Ok(old_cwd) = old_cwd {
            if let Some(dirname) = old_cwd.file_name() {
                if let Some(idx) = self.ls_output_buf.iter()
                    .position(|x| std::ffi::OsString::from(x) == dirname) {
                    self.move_cursor(idx as i32);
                }
            }
        }
        Ok(())
    }

    pub fn is_searching(&self) -> bool {
        !self.search_state.search_string.is_empty()
    }

    pub fn search_string(&self) -> &String {
        &self.search_state.search_string
    }

    /// The current search matches
    pub fn search_matches(&self) -> &MatchesType {
        &self.search_state.matches
    }

    /// Move the cursor to the next or previous match in the current list of
    /// matches, and update the head of the match list to point to the new current
    /// value. If dir is positive, move to the next match, if it's negative, move
    /// to the previous match, and if it's zero, move to the cursor to the current
    /// match (without modifying the match list).
    pub fn move_cursor_to_adjacent_match(&mut self, dir: i32) {
        if self.search_state.matches.len() > 0 && self.is_searching() {
            if dir < 0 {
                self.search_state.matches.decrement();
            } else if dir > 0 {
                self.search_state.matches.increment();
            }

            let (i, _) = self.search_state.matches.current_item().unwrap();
            let i = i.clone() as u32;
            self.move_cursor_to(i);
        }
    }

    /// Update the matches and the cursor position
    fn on_search_string_changed(&mut self) {
        self.search_state.update_matches(&self.ls_output_buf);
        self.move_cursor_to_adjacent_match(0);
    }

    pub fn advance_search(&mut self, query: &str) {
        self.search_state.search_string.push_str(query);
        self.on_search_string_changed();
    }

    pub fn erase_search_char(&mut self) {
        if let Some(_) = self.search_state.search_string.pop() {
            //TODO: keep cursor position. now if we're at the second match and type backspace, the
            //curor jumps back to the first
            self.search_state.update_matches(&self.ls_output_buf);
            self.on_search_string_changed();
        };
    }

    pub fn clear_search(&mut self) {
        self.search_state.clear();
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_filenames(n: u32) -> Vec<String> {
        (1..=n).map(|i| format!("file {}", i)).collect()
    }

    fn create_test_state(win_h: u32, n_filenames: u32) -> TereAppState {
        TereAppState {
            cursor_pos: 0,
            scroll_pos: 0,
            main_win_h: win_h,
            main_win_w: 10,
            ls_output_buf: create_test_filenames(n_filenames),
            header_msg: "".into(),
            info_msg: "".into(),
            search_state: Default::default(),
        }
    }

    #[test]
    fn test_scrolling_bufsize_less_than_window_size() {
        let mut state = create_test_state(10, 4);

        for i in 1..=3 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, 0);
        }

        state.move_cursor(100);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);

        for i in 1..=3 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 3 - i);
            assert_eq!(state.scroll_pos, 0);
        }

        assert_eq!(state.cursor_pos, 0);

        for _ in 0..5 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, 0);
        }

        state.move_cursor(-100);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        // test jumping all the way to the bottom and back
        state.move_cursor(100);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }

    #[test]
    fn test_scrolling_bufsize_equal_to_window_size() {
        let mut state = create_test_state(4, 4);

        for i in 1..=3 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        for _ in 0..5 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, 0);
        }

        for i in 1..=3 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 3-i);
            assert_eq!(state.scroll_pos, 0);
        }

        // test jumping all the way to the bottom and back
        state.move_cursor(100);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 0);
        state.move_cursor(-100);
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
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        // scroll to the end of the list
        for i in 1..=max_scroll {
            println!("scrolling beyond screen {}, cursor at {}, scroll {}",
                     i, state.cursor_pos, state.scroll_pos);
            state.move_cursor(1);
            println!("after move: cursor at {}, scroll {}",
                     state.cursor_pos, state.scroll_pos);
            assert_eq!(state.cursor_pos, max_cursor);
            assert_eq!(state.scroll_pos, i);
        }

        assert_eq!(state.scroll_pos, max_scroll);

        // check that nothing changes when trying to scroll further
        for _ in 0..5 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, max_cursor);
            assert_eq!(state.scroll_pos, max_scroll);
        }
        state.move_cursor(win_h as i32 + 100);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the window
        for i in 1..=max_cursor {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, max_cursor-i);
            assert_eq!(state.scroll_pos, max_scroll);
        }
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, max_scroll);

        // scroll back to the top of the list
        for i in 1..=max_scroll {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, max_scroll-i);
        }

        // check that nothing changes when trying to scroll further
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
        for _ in 0..5 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, 0);
        }
        state.move_cursor(-100);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);

        // test jumping all the way to the bottom and back
        state.move_cursor(win_h as i32 + 100);
        assert_eq!(state.cursor_pos, max_cursor);
        assert_eq!(state.scroll_pos, max_scroll);
        state.move_cursor(-100 - win_h as i32);
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
}
