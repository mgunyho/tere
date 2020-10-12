use pancurses::{initscr, endwin, noecho, Input, curs_set};
use std::convert::TryInto;

const HEADER_SIZE: i32 = 1;
const INFO_WIN_SIZE: i32 = 1;

mod app_state;
use app_state::TereAppState;

/// This struct groups together ncurses windows for the main content, header and
/// footer, and an application state object
struct TereTui {
    header_win: pancurses::Window,
    info_win: pancurses::Window,
    //footer_win: pancurses::Window, //TODO
    main_win: pancurses::Window,
    app_state: TereAppState,
}

impl TereTui {
    /// Helper function for (re)creating the main window
    pub fn create_main_window(root_win: &pancurses::Window)
        -> pancurses::Window {
        root_win.subwin(root_win.get_max_y() - HEADER_SIZE - INFO_WIN_SIZE, 0,
                        HEADER_SIZE, 0)
                .expect("failed to create main window!")
    }

    /// Helper function for (re)creating the header window
    pub fn create_header_window(root_win: &pancurses::Window)
        -> pancurses::Window {
        let header = root_win.subwin(HEADER_SIZE, 0, 0, 0)
                .expect("failed to create header window!");

        //TODO: make header bg/font color configurable via settings
        header.attrset(pancurses::Attribute::Bold);
        header
    }

    pub fn create_info_window(root_win: &pancurses::Window)
        -> pancurses::Window {
        let infobox = root_win.subwin(INFO_WIN_SIZE, 0,
                        root_win.get_max_y() - INFO_WIN_SIZE, 0)
                        .expect("failed to create info window!");
        infobox.attrset(pancurses::Attribute::Bold);
        infobox
    }

    pub fn init(root_win: &pancurses::Window) -> Self {
        let main_win = Self::create_main_window(root_win);
        let state = TereAppState::init(
            main_win.get_max_x().try_into().unwrap_or(1),
            main_win.get_max_y().try_into().unwrap_or(1)
        );
        let mut ret = Self {
            header_win: Self::create_header_window(root_win),
            main_win: main_win,
            info_win: Self::create_info_window(root_win),
            app_state: state,
        };

        ret.update_header();
        ret.redraw_info_window();
        ret.redraw_main_window();
        return ret;
    }

    pub fn redraw_header(&mut self) {
        self.header_win.clear();
        //TODO: what to do if window is narrower than path?
        // add "..." to beginning? or collapse folder names? make configurable?
        self.header_win.mvaddstr(0, 0, &self.app_state.header_msg);
        self.header_win.refresh();
    }

    pub fn update_header(&mut self) {
        self.app_state.update_header();
        self.redraw_header();
    }

    pub fn redraw_info_window(&self) {
        self.info_win.clear();
        self.info_win.mvaddstr(0, 0, &self.app_state.info_msg);
        self.info_win.refresh();
    }

    pub fn info_message(&mut self, msg: &str) {
        self.app_state.info_msg = msg.to_string();
        self.info_win.attrset(pancurses::Attribute::Bold);
        self.redraw_info_window();
    }

    pub fn error_message(&mut self, msg: &str) {
        //TODO: red color (also: make it configurable)
        let mut error_msg = String::from("error: ");
        error_msg.push_str(msg);
        self.info_message(&error_msg);
    }

    fn change_row_attr(&self, row: u32, attr: pancurses::chtype) {
        let (_, color_pair) = self.main_win.attrget();
        self.main_win.mvchgat(row as i32, 0, -1, attr, color_pair);
    }

    pub fn unhighlight_row(&self, row: u32) {
        self.change_row_attr(row, pancurses::Attribute::Normal.into());
    }

    pub fn highlight_row(&self, row: u32) {
        // Highlight the row `row` in the main window. Row 0 is the first row of
        // the main window
        self.change_row_attr(row, pancurses::A_STANDOUT);
    }

    pub fn redraw_main_window(&self) {
        self.main_win.clear();
        let (max_y, max_x) = self.main_win.get_max_yx();
        let scroll_pos = self.app_state.scroll_pos;
        for (i, line) in self.app_state.ls_output_buf.iter().skip(scroll_pos as usize)
            .enumerate().take(max_y as usize) {
            self.main_win.mvaddnstr(i as i32, 0, line, max_x);
        }

        self.highlight_row(self.app_state.cursor_pos);

        self.main_win.refresh();
    }

    /// Update the app state by moving the cursor by the specified amount, and
    /// redraw the view as necessary.
    pub fn move_cursor(&mut self, amount: i32) {

        self.unhighlight_row(self.app_state.cursor_pos);

        let old_scroll_pos = self.app_state.scroll_pos;

        self.app_state.move_cursor(amount);

        if self.app_state.scroll_pos != old_scroll_pos {
            // redraw_main_window takes care of highlighting the cursor row
            // and refreshing
            self.redraw_main_window();
        } else {
            self.highlight_row(self.app_state.cursor_pos);
            self.main_win.refresh();
        }

    }

    pub fn change_dir(&mut self, path: &str) {
        match self.app_state.change_dir(path) {
            Err(e) => {
                if cfg!(debug_assertions) {
                    self.error_message(&format!("{:?}", e));
                } else {
                    self.error_message(&format!("{}", e));
                }
            },
            Ok(()) => {
                self.update_header();
                self.redraw_main_window();
            }
        }
    }

    pub fn on_resize(&mut self, root_win: &pancurses::Window) {
        //TODO: see https://github.com/ihalila/pancurses/pull/65
        // it's not possible to resize windows with pancurses ATM,
        // so we have to hack around and destroy/recreate the main
        // window every time. Doesn't seem to be too much of a
        // performance issue.
        self.main_win = Self::create_main_window(root_win);
        self.header_win = Self::create_header_window(root_win);
        self.info_win = Self::create_info_window(root_win);

        let (h, w) = self.main_win.get_max_yx();
        let (h, w) = (h as u32, w as u32);
        self.app_state.update_main_window_dimensions(w, h);

        self.redraw_header();
        self.redraw_info_window();
        self.redraw_main_window();
    }

    pub fn main_event_loop(&mut self, root_win: &pancurses::Window) {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
                Some(Input::KeyUp) => {
                    self.move_cursor(-1);
                }
                Some(Input::KeyDown) => {
                    self.move_cursor(1);
                }
                Some(Input::KeyRight) => {
                    self.change_dir("");
                }
                Some(Input::KeyLeft) => {
                    self.change_dir("..");
                }
                Some(Input::Character('\x1B')) => {
                    // Either ESC or ALT+key. If it's ESC, the next getch will be
                    // err. If it's ALT+key, next getch will contain the key
                    root_win.nodelay(true);
                    match root_win.getch() {
                        Some(Input::Character(c)) => { self.info_message(&format!("ALT+{}", c)); },
                        _ => { break; },
                    }
                    root_win.nodelay(false);
                }
                Some(Input::KeyDC) | Some(Input::Character('q')) => break,
                Some(Input::Character(c)) => {
                    //TODO: type to search (use separate footer window for that)
                    self.info_message(&format!("{}", c));
                },
                Some(Input::KeyResize) => { self.on_resize(root_win) },
                Some(input) => { self.info_message(&format!("{:?}", input)); },
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

    ui.main_event_loop(&root_window);

    endwin();
}
