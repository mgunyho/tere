use pancurses::{initscr, endwin, noecho, Input, curs_set};
use ncurses;
use std::convert::{From, TryInto};

use clap::{App, Arg, ArgMatches};

const HEADER_SIZE: i32 = 1;
const INFO_WIN_SIZE: i32 = 1;
const FOOTER_SIZE: i32 = 1;

//TODO: rustfmt
//TODO: clippy

mod app_state;
use app_state::TereAppState;

#[derive(Debug)]
enum TereError {
    WindowInit(String, i32),
    IoError(std::io::Error),
}

impl From<std::io::Error> for TereError {
    fn from(e: std::io::Error) -> Self {
        Self::IoError(e)
    }
}

/// This struct groups together ncurses windows for the main content, header and
/// footer, and an application state object
struct TereTui {
    header_win: pancurses::Window,
    info_win: pancurses::Window,
    footer_win: pancurses::Window,
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
        Self::subwin_helper(
            root_win,
            root_win.get_max_y() - HEADER_SIZE - INFO_WIN_SIZE - FOOTER_SIZE,
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
            -INFO_WIN_SIZE - FOOTER_SIZE,
            "info")?;
        infobox.attrset(pancurses::Attribute::Bold);
        Ok(infobox)
    }

    pub fn create_footer_window(root_win: &pancurses::Window)
        -> Result<pancurses::Window, TereError> {
        let footer = Self::subwin_helper(root_win, FOOTER_SIZE, -FOOTER_SIZE,
                                         "footer")?;
        footer.attrset(pancurses::Attribute::Bold);
        Ok(footer)
    }

    pub fn init(args: &ArgMatches,
                root_win: &pancurses::Window) -> Result<Self, TereError> {
        let main_win = Self::create_main_window(root_win)?;
        let state = TereAppState::init(
            args,
            main_win.get_max_x().try_into().unwrap_or(1),
            main_win.get_max_y().try_into().unwrap_or(1)
        );
        let mut ret = Self {
            header_win: Self::create_header_window(root_win)?,
            main_win: main_win,
            info_win: Self::create_info_window(root_win)?,
            footer_win: Self::create_footer_window(root_win)?,
            app_state: state,
        };

        ret.update_header();
        ret.redraw_all_windows();
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
        // TODO: consider removing redraw here... (is inconsistent with the rest of the 'update' functions)
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

    pub fn redraw_footer(&self) {
        self.footer_win.clear();
        let mut extra_msg = String::new();
        if self.app_state.is_searching() {
            self.footer_win.mvaddstr(0, 0, &self.app_state.search_string());
            extra_msg.push_str(&format!("{} / {} / {}",
                               self.app_state.search_matches()
                                   .current_pos().map(|i| i + 1).unwrap_or(0),
                               self.app_state.search_matches().len(),
                               self.app_state.ls_output_buf.len()));
        } else {
            //TODO: show no. of files/folders separately? like 'n folders, n files'
            let cursor_idx = self.app_state.cursor_pos +
                             self.app_state.scroll_pos + 1;
            extra_msg.push_str(&format!("{} / {}",
                               cursor_idx,
                               self.app_state.ls_output_buf.len()));
        }
        self.footer_win.mvaddstr(0,
                                 self.main_win.get_max_x() - extra_msg.len() as i32,
                                 extra_msg);
        self.footer_win.refresh();
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

    pub fn highlight_row_exclusive(&self, row: u32) {
        // Highlight the row `row` exclusively, and hide all other rows.
        // Note that refresh() needs to be called externally.
        let row_content: &str = self.app_state.ls_output_buf
            .get((row + self.app_state.scroll_pos) as usize)
            .map(|s| s.as_ref())
            .unwrap_or("");

        self.main_win.clear();
        self.main_win.mvaddstr(row as i32, 0, row_content);
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

        // highlight matches that are in view
        let is_in_view = |i: usize| {
            let i = i as u32;
            let above = i.checked_sub(scroll_pos).unwrap_or(0) < max_y as u32;
            let below = scroll_pos <= i;
            above && below
        };
        //TODO: search_anywhere...
        let match_range = 0..self.app_state.search_string().len();
        self.app_state.search_matches().iter()
            .filter(|(i, _)| is_in_view(*i))
            .map(|(i, _)| *i as i32 - scroll_pos as i32) // map indices to cursor positions
            .for_each(|i| {
                self.main_win.mv(i, match_range.start as i32);
                let (_, color_pair) = self.main_win.attrget();
                self.main_win.chgat(match_range.len() as i32,
                                    pancurses::Attribute::Underline.into(),
                                    color_pair);
            });

        self.main_win.refresh();
    }

    fn redraw_all_windows(&mut self) {
        self.redraw_header();
        self.redraw_info_window();
        self.redraw_footer();
        self.redraw_main_window();
    }

    /// Update the app state by moving the cursor by the specified amount, and
    /// redraw the view as necessary.
    pub fn move_cursor(&mut self, amount: i32, wrap: bool) {

        //TODO: moving cursor removes highlights
        // (in principle. currently on_arrow_key redraws the whole screen so this
        // is not a problem)
        self.unhighlight_row(self.app_state.cursor_pos);

        let old_scroll_pos = self.app_state.scroll_pos;

        self.app_state.move_cursor(amount, wrap);

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
                self.info_message("");
            }
        }
        self.redraw_main_window();
        self.redraw_footer();
    }

    pub fn on_search_char(&mut self, c: char) {
        self.app_state.advance_search(&c.to_string());
        if self.app_state.search_matches().len() == 1 {
            // There's only one match, highlight it and then change dir
            self.highlight_row_exclusive(self.app_state.cursor_pos);
            self.main_win.refresh();

            //TODO: make duration configurable
            std::thread::sleep(std::time::Duration::from_millis(200));
            pancurses::flushinp(); // ignore keys pressed during sleep

            self.change_dir("");
        }
        self.redraw_main_window();
        self.redraw_footer();
    }

    pub fn erase_search_char(&mut self) {
        self.app_state.erase_search_char();
        self.redraw_main_window();
        self.redraw_footer();
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
        self.footer_win = Self::create_footer_window(root_win)?;

        let (h, w) = self.main_win.get_max_yx();
        let (h, w) = (h as u32, w as u32);
        self.app_state.update_main_window_dimensions(w, h);

        self.redraw_all_windows();
        Ok(())
    }

    pub fn on_arrow_key(&mut self, up: bool) {
        let dir = if up { -1 } else { 1 };
        if self.app_state.is_searching() {
            //TODO: handle case where 'is_searching' but there are no matches - move cursor?
            self.app_state.move_cursor_to_adjacent_match(dir);
            self.redraw_main_window();
        } else {
            self.move_cursor(dir, true);
        }
        self.redraw_footer();
    }

    // When the 'page up' or 'page down' keys are pressed
    pub fn on_page_up_down(&mut self, up: bool) {
        if !self.app_state.is_searching() {
            let (h, _) = self.main_win.get_max_yx();
            let mut delta = 2 * h - 3;
            if up {
                delta *= -1;
            }
            self.move_cursor(- (self.app_state.cursor_pos as i32) + delta, false);
            self.redraw_footer();
        } //TODO: how to handle page up / page down while searching? jump to the next match below view?
    }

    pub fn main_event_loop(&mut self, root_win: &pancurses::Window) -> Result<(), TereError> {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
                //TODO: home/pg up / pg dn keys
                Some(Input::KeyUp) => {
                    self.on_arrow_key(true);
                },
                Some(Input::KeyDown) => {
                    self.on_arrow_key(false);
                },
                Some(Input::KeyRight) | Some(Input::Character('\n')) => {
                    self.change_dir("");
                },
                Some(Input::KeyLeft) => {
                    self.change_dir("..");
                },
                Some(Input::KeyPPage) => {
                    self.on_page_up_down(true);
                },
                Some(Input::KeyNPage) => {
                    self.on_page_up_down(false);
                },
                Some(Input::KeyHome) => {
                    if !self.app_state.is_searching() {
                        self.app_state.move_cursor_to(0);
                        self.redraw_main_window();
                    } // TODO: jump to first match
                },
                Some(Input::KeyEnd) => {
                    if !self.app_state.is_searching() {
                        let end = self.app_state.ls_output_buf.len() as u32;
                        self.app_state.move_cursor_to(end);
                        self.redraw_main_window();
                    } // TODO: jump to last match
                },
                Some(Input::Character('\x1B')) => {
                    // Either ESC or ALT+key. If it's ESC, the next getch will be
                    // err. If it's ALT+key, next getch will contain the key
                    root_win.nodelay(true);
                    match root_win.getch() {
                        //TODO: alt+home -> go to home folder (possible?)
                        // TODO: unify these with the  arrow keys above, with custom 'getch_with_alt' function or something that returns an enum which makes it possible to match alt once
                        Some(Input::Character('k')) => self.on_arrow_key(true),
                        Some(Input::Character('j')) => self.on_arrow_key(false),
                        Some(Input::Character('h')) => self.change_dir(".."),
                        Some(Input::Character('l')) => self.change_dir(""),
                        Some(Input::Character('u')) => self.on_page_up_down(true),
                        Some(Input::Character('d')) => self.on_page_up_down(false),

                        Some(Input::Character(c)) => { self.info_message(&format!("ALT+{}", c)); },  //TODO: alt+up arrow -> move up (possible?)
                        None => {
                            if self.app_state.is_searching() {
                                self.app_state.clear_search();
                                self.redraw_main_window();
                                self.redraw_footer();
                            } else {
                                break;
                            }
                        },
                        _ => (),
                    }
                    root_win.nodelay(false);
                },
                Some(Input::KeyDC) => break,
                Some(Input::Character(c)) => {
                    self.on_search_char(c);
                },
                Some(Input::KeyBackspace) => {
                    self.erase_search_char();
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

    let cli_args = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        //.author(env!("CARGO_PKG_AUTHORS")) // TODO: rest of these https://stackoverflow.com/a/27841363
        .arg(Arg::with_name("folders-only")
             .long("folders-only")
             //.short("f")  // TODO: check conflicts
             .help("only show folders in listing")
             )
        .get_matches();

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

    let res = TereTui::init(&cli_args, &root_window)
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
