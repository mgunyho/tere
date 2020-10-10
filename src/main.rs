use pancurses::{initscr, endwin, noecho, Input, curs_set};
use std::convert::TryInto;

const HEADER_SIZE: i32 = 1;

mod app_state;
use app_state::TereAppState;

/// This struct groups together ncurses windows for the main content, header and
/// footer, and an application state object
struct TereTui {
    header_win: pancurses::Window,
    //footer_win: pancurses::Window, //TODO
    main_win: pancurses::Window,
    app_state: TereAppState,
}

impl TereTui {
    pub fn init(root_win: &pancurses::Window) -> Self {
        let main_win = root_win
                .subwin(root_win.get_max_y() - HEADER_SIZE, 0, HEADER_SIZE, 0)
                .expect("failed to initialize main window!");
        let state = TereAppState::init(
            main_win.get_max_x().try_into().unwrap_or(1),
            main_win.get_max_y().try_into().unwrap_or(1)
        );
        let ret = Self {
            header_win: root_win.subwin(HEADER_SIZE, 0, 0, 0)
                .expect("failed to initialize header window!"),
            main_win: main_win,
            app_state: state,
        };

        ret.init_header();
        ret.update_header();
        ret.redraw_main_window();
        return ret;
    }

    pub fn init_header(&self) {
        //TODO: make header bg/font color configurable via settings
        self.header_win.attrset(pancurses::A_BOLD);
    }

    pub fn update_header(&self) {
        //TODO: move this to app state
        //TODO: add another row to header (or footer?) with info, like 'tere - type ALT+? for help', and show status message when trying to open file etc

        let cwd: std::string::String = match std::env::current_dir() {
            Ok(path) => format!("{}", path.display()),
            Err(e) => format!("Unable to get current dir! ({})", e),
        };

        self.header_win.addstr(cwd);
        self.header_win.refresh();
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
            .enumerate() .take(max_y as usize) {
            self.main_win.mvaddnstr(i as i32, 0, line, max_x);
        }

        self.highlight_row(self.app_state.cursor_pos);

        self.main_win.refresh();
    }

    pub fn main_event_loop(&mut self, root_win: pancurses::Window) {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
                Some(Input::KeyUp) => {
                    self.app_state.scroll_pos = self.app_state.scroll_pos.checked_sub(1)
                        .unwrap_or(0);
                    self.redraw_main_window();
                }
                Some(Input::KeyDown) => {
                    self.app_state.scroll_pos += 1;
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
                Some(Input::KeyResize) => { self.redraw_main_window(); },
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

    ui.main_event_loop(root_window);

    endwin();
}
