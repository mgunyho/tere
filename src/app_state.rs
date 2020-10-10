
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
}