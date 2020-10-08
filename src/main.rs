use pancurses::{initscr, endwin, noecho, Input, curs_set};

const HEADER_SIZE: i32 = 1;


struct TereTui {
    header_win: pancurses::Window,
    //footer_win: pancurses::Window, //TODO
    main_win: pancurses::Window,
    // This vector will hold the list of files/folders in the current directory
    ls_output_buf: Vec<String>,

    // the row on which the cursor is currently on, counted starting from the
    // start of `ls_output_buf` (not from the top of the screen).
    cursor_pos: u32,

    // the top of the screen corresponds to this row in the `ls_output_buf`.
    scroll_pos: u32,

    //TODO
    //search_string: String,
    //// if this is false, match anywhere, otherwise match only from the beginning
    //search_anywhere: bool,
}

impl TereTui {
    pub fn init(root_win: &pancurses::Window) -> Self {
        let mut ret = Self {
            header_win: root_win.subwin(HEADER_SIZE, 0, 0, 0)
                .expect("failed to initialize header window!"),
            main_win: root_win
                .subwin(root_win.get_max_y() - HEADER_SIZE, 0, HEADER_SIZE, 0)
                .expect("failed to initialize main window!"),
            ls_output_buf: vec![],
            cursor_pos: 0, // TODO: get last value from previous run
            scroll_pos: 0,
            //search_string: "".into(),
            //search_anywhere: false,
        };

        ret.init_header();
        ret.update_header();
        ret.update_ls_output_buf();
        ret.redraw_main_window();
        return ret;
    }

    pub fn init_header(&self) {
        //TODO: make header bg/font color configurable via settings
        self.header_win.attrset(pancurses::A_BOLD);
    }

    pub fn update_header(&self) {
        //TODO: add another row to header with info, like 'tere - type ALT+? for help', and show status message when trying to open file etc

        let cwd: std::string::String = match std::env::current_dir() {
            Ok(path) => format!("{}", path.display()),
            Err(e) => format!("Unable to get current dir! ({})", e),
        };

        self.header_win.addstr(cwd);
        self.header_win.refresh();
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

    pub fn higlight_row(&self, row: u32) {
        // Highlight the row `row` in the main window. Row 0 is the first row of
        // the main window
        //TODO
        let (_, color_pair) = self.main_win.attrget();
        self.main_win.mvchgat(row as i32, 0, -1, pancurses::A_STANDOUT,
                              color_pair);
    }

    pub fn redraw_main_window(&self) {
        self.main_win.clear();
        let (max_y, max_x) = self.main_win.get_max_yx();
        for (i, line) in self.ls_output_buf.iter().skip(self.scroll_pos as usize)
            .enumerate() .take(max_y as usize) {
            self.main_win.mvaddnstr(i as i32, 0, line, max_x);
        }

        self.higlight_row(self.cursor_pos);

        self.main_win.refresh();
    }

    pub fn main_loop(&mut self, root_win: pancurses::Window) {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
                Some(Input::KeyUp) => {
                    self.scroll_pos = self.scroll_pos.checked_sub(1)
                        .unwrap_or(0);
                    self.redraw_main_window();
                }
                Some(Input::KeyDown) => {
                    self.scroll_pos += 1;
                    self.redraw_main_window();
                }
                Some(Input::Character('\x1B')) => {
                    // Either ESC or ALT+key. If it's ESC, the next getch will be
                    // err. If it's ALT+key, next getch will contain the key
                    root_win.nodelay(true);
                    match root_win.getch() {
                        Some(Input::Character(c)) => { self.main_win.addstr(&format!("ALT+{}", c)); },
                        _ => { break; },
                    }
                    root_win.nodelay(false);
                }
                Some(Input::KeyDC) => break,
                Some(Input::Character(c)) => { self.main_win.addstr(&format!("{}", c)); },
                Some(Input::KeyResize) => (),
                Some(input) => { self.main_win.addstr(&format!("{:?}", input)); },
                None => (),
            }
            self.main_win.refresh();
        }
    }
}


fn main() {
    let root_window = initscr();

    root_window.keypad(true); // enable arrow keys etc
    curs_set(0);

    let mut ui = TereTui::init(&root_window);

    noecho();

    ui.main_loop(root_window);

    endwin();
}
