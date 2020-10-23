use pancurses::{initscr, endwin, noecho, Input, curs_set};
use ncurses;
use std::convert::TryInto;

const HEADER_SIZE: i32 = 1;
const INFO_WIN_SIZE: i32 = 1;

//TODO: rustfmt
//TODO: clippy

mod app_state;
use app_state::TereAppState;

#[derive(Debug)]
enum TereError {
    WindowInit(String, i32),
}

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

    // Helper function for creating subwindows. if `begy` is negative, it is counted
    // bacwards from `root_win.get_max_y()`.
    fn subwin_helper(root_win: &pancurses::Window,
                     nlines: i32,
                     begy: i32,
                     label: &str,
                     ) -> Result<pancurses::Window, TereError> {

        let begy = if begy < 0 { root_win.get_max_y() + begy } else { begy };
        root_win.subwin(nlines, 0, begy, 0)
            .map_err(|e| TereError::WindowInit(
                format!("failed to create {} window!", label), e)
            )
    }

    /// Helper function for (re)creating the main window
    pub fn create_main_window(root_win: &pancurses::Window)
        -> Result<pancurses::Window, TereError> {
        Self::subwin_helper(root_win,
                            root_win.get_max_y() - HEADER_SIZE - INFO_WIN_SIZE,
                            HEADER_SIZE,
                            "main")
    }

    /// Helper function for (re)creating the header window
    pub fn create_header_window(root_win: &pancurses::Window)
        -> Result<pancurses::Window, TereError> {
        let header = Self::subwin_helper(root_win, HEADER_SIZE, 0, "header")?;

        //TODO: make header bg/font color configurable via settings
        header.attrset(pancurses::Attribute::Bold);
        Ok(header)
    }

    pub fn create_info_window(root_win: &pancurses::Window)
        -> Result<pancurses::Window, TereError> {
        let infobox = Self::subwin_helper(
            root_win,
            INFO_WIN_SIZE,
            - INFO_WIN_SIZE,
            "info")?;
        infobox.attrset(pancurses::Attribute::Bold);
        Ok(infobox)
    }

    pub fn init(root_win: &pancurses::Window) -> Result<Self, TereError> {
        let main_win = Self::create_main_window(root_win)?;
        let state = TereAppState::init(
            main_win.get_max_x().try_into().unwrap_or(1),
            main_win.get_max_y().try_into().unwrap_or(1)
        );
        let mut ret = Self {
            header_win: Self::create_header_window(root_win)?,
            main_win: main_win,
            info_win: Self::create_info_window(root_win)?,
            app_state: state,
        };

        ret.update_header();
        ret.redraw_info_window();
        ret.redraw_main_window();
        Ok(ret)
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

    /// Set/update the current info message and redraw the info window
    pub fn info_message(&mut self, msg: &str) {
        //TODO: add thread with timeout that will clear the info message after x seconds?
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

    pub fn on_resize(&mut self, root_win: &pancurses::Window) -> Result<(), TereError> {
        //TODO: see https://github.com/ihalila/pancurses/pull/65
        // it's not possible to resize windows with pancurses ATM,
        // so we have to hack around and destroy/recreate the main
        // window every time. Doesn't seem to be too much of a
        // performance issue.
        self.main_win = Self::create_main_window(root_win)?;
        self.header_win = Self::create_header_window(root_win)?;
        self.info_win = Self::create_info_window(root_win)?;

        let (h, w) = self.main_win.get_max_yx();
        let (h, w) = (h as u32, w as u32);
        self.app_state.update_main_window_dimensions(w, h);

        self.redraw_header();
        self.redraw_info_window();
        self.redraw_main_window();
        Ok(())
    }

    pub fn main_event_loop(&mut self, root_win: &pancurses::Window) -> Result<(), TereError> {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
                Some(Input::KeyUp) => {
                    self.move_cursor(-1);
                }
                Some(Input::KeyDown) => {
                    self.move_cursor(1);
                }
                Some(Input::KeyRight) | Some(Input::Character('\n')) => {
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
                        None => { break; },
                        _ => (),
                    }
                    root_win.nodelay(false);
                }
                Some(Input::KeyDC) => break,
                Some(Input::Character(c)) => {
                    //TODO: type to search (use separate footer window for that)
                    self.info_message(&format!("{}", c));
                },
                Some(Input::KeyResize) => { self.on_resize(root_win)? },
                Some(input) => { self.info_message(&format!("{:?}", input)); },
                None => (),
            }
            self.main_win.refresh();
        }

        Ok(())
    }
}

fn main() {
    let root_window = initscr();

    ncurses::set_escdelay(0);
    root_window.keypad(true); // enable arrow keys etc
    curs_set(0);

    noecho();

    let prepend_err = |msg: &str, e: TereError| {
        match e {
            TereError::WindowInit(desc, code) => {
                TereError::WindowInit(msg.to_string() + &desc, code)
            }
        }
    };

    let res = TereTui::init(&root_window)
        .map_err(|e| prepend_err("error in initializing UI: ", e))
        .and_then(|mut ui| ui.main_event_loop(&root_window)
            .map_err(|e| prepend_err("error in main event loop: ", e))
        );

    // clean up even if there was an error
    endwin();

    // panic if there was an error
    res.unwrap();

    // no error, print cwd
    let cwd = std::env::current_dir().expect("error getting cwd");
    println!("{}", cwd.display());
}
