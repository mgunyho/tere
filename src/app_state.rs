
/// This struct represents the state of the application. Note that it has no
/// notion of curses windows.
pub struct TereAppState {

    // Width and height of the main window. Their values have to be updated
    // externally if they change.
    main_win_w: u32,
    main_win_h: u32,

    // This vector will hold the list of files/folders in the current directory,
    // including ".." (the parent folder).
    pub ls_output_buf: Vec<String>,

    // the row on which the cursor is currently on, counted starting from the
    // top of the screen (not from the start of `ls_output_buf`). Note that this
    // doesn't have anything to do with the ncurses curspor position.
    pub cursor_pos: u32,

    // The top of the screen corresponds to this row in the `ls_output_buf`.
    pub scroll_pos: u32,

    //TODO
    //search_string: String,
    //// if this is false, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool,

    //header_msg: String, //TODO
    //info_msg: String, //TODO
    //footer_extra_msg: String, //TODO
}

impl TereAppState {
    pub fn init(window_w: u32, window_h: u32) -> Self {
        let mut ret = Self {
            main_win_w: window_w,
            main_win_h: window_h,
            ls_output_buf: vec![],
            cursor_pos: 0, // TODO: get last value from previous run
            scroll_pos: 0,
            //search_string: "".into(),
            //search_anywhere: false,
        };

        //ret.update_header();  //TODO: move this here
        ret.update_ls_output_buf();
        return ret;
    }

    pub fn update_main_window_dimensions(&mut self, w: u32, h: u32) {
        self.main_win_w = w;
        self.main_win_h = h;
    }

    pub fn update_ls_output_buf(&mut self) {
        if let Ok(entries) = std::fs::read_dir(".") {
            self.ls_output_buf.clear();
            self.ls_output_buf.extend(
                //TODO: sorting...
                entries.filter_map(|e| e.ok())
                    .map(|e| e.file_name().into_string().ok())
                    .filter_map(|e| e)
            );
        }
        //TODO: show error message (add separate msg box)
    }

    /// Move the cursor up (positive amount) or down (negative amount), and scroll
    /// the view as necessary
    pub fn move_cursor(&mut self, amount: i32) {

        let old_cursor_pos = self.cursor_pos;
        let old_scroll_pos = self.scroll_pos;
        let ls_buf_size = self.ls_output_buf.len() as u32;
        let max_y = self.main_win_h;

        let new_cursor_pos: i32 = old_cursor_pos as i32 + amount;

        // the new cursor position in coordinates relative to the start of `ls_output_buf`
        //let new_buf_pos = (old_scroll_pos + old_scroll_pos) as i32 + amount

        if new_cursor_pos < 0 {
            // attempting to go above the current view, scroll up
            self.scroll_pos = self.scroll_pos
                .checked_sub(new_cursor_pos.abs() as u32).unwrap_or(0);
            self.cursor_pos = 0;
        } else if new_cursor_pos as u32 + old_scroll_pos >= ls_buf_size {
            // attempting to go below content
            self.scroll_pos = ls_buf_size.checked_sub(max_y).unwrap_or(0);
            self.cursor_pos = ls_buf_size - self.scroll_pos - 1;
        } else if new_cursor_pos as u32 >= max_y {
            // attempting to go below current view, scroll down
            // the new scroll position should satisfy old_scroll_position + amount  = ???
            //TODO
            self.scroll_pos = new_cursor_pos as u32 - max_y + old_scroll_pos;
            self.cursor_pos = max_y - 1;
        } else {
            self.cursor_pos = new_cursor_pos as u32;
        }

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

    #[test]
    fn test_scrolling_bufsize_larger_than_window_size() {
        let mut state = create_test_state(4, 10);

        // move cursor all the way to the bottom of the window
        for i in 1..=3 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, i);
            assert_eq!(state.scroll_pos, 0);
        }

        // scroll to the end of the list
        for i in 1..=6 {
            println!("scrolling beyond screen {}, cursor at {}, scroll {}",
                     i, state.cursor_pos, state.scroll_pos);
            state.move_cursor(1);
            println!("after move: cursor at {}, scroll {}",
                     state.cursor_pos, state.scroll_pos);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, i);
        }

        assert_eq!(state.scroll_pos, 6);

        // check that nothing changes when trying to scroll further
        for _ in 0..5 {
            state.move_cursor(1);
            assert_eq!(state.cursor_pos, 3);
            assert_eq!(state.scroll_pos, 6);
        }
        state.move_cursor(100);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 6);

        // scroll back to the top of the window
        for i in 1..=3 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 3-i);
            assert_eq!(state.scroll_pos, 6);
        }
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 6);

        // scroll back to the top of the list
        for i in 1..=6 {
            state.move_cursor(-1);
            assert_eq!(state.cursor_pos, 0);
            assert_eq!(state.scroll_pos, 6-i);
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
        state.move_cursor(100);
        assert_eq!(state.cursor_pos, 3);
        assert_eq!(state.scroll_pos, 6);
        state.move_cursor(-100);
        assert_eq!(state.cursor_pos, 0);
        assert_eq!(state.scroll_pos, 0);
    }
}
