use pancurses::{initscr, endwin, noecho, Input};

const HEADER_SIZE: i32 = 1;


struct TereTui {
    header_win: pancurses::Window,
    //footer_win: pancurses::Window, //TODO
    main_win: pancurses::Window,
    // This vector will hold the list of files/folders in the current directory
    ls_output_buf: Vec<String>,
}

impl TereTui {
    pub fn init(root_win: &pancurses::Window) -> Self {
        let ret = Self {
            header_win: root_win.subwin(HEADER_SIZE, 0, 0, 0)
                .expect("failed to initialize header window!"),
            main_win: root_win
                .subwin(root_win.get_max_y() - HEADER_SIZE, 0, HEADER_SIZE, 0)
                .expect("failed to initialize main window!"),
            ls_output_buf: vec![],
        };

        ret.init_header();
        ret.update_header();
        ret.update_main_window();
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

    pub fn update_main_window(&self) {
        //TODO
        self.main_win.printw("hëllö");
        self.main_win.refresh();
    }

    pub fn main_loop(&self, root_win: pancurses::Window) {
        // root_win is the window created by initscr()
        loop {
            match root_win.getch() {
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

    let ui = TereTui::init(&root_window);

    noecho();

    ui.main_loop(root_window);

    endwin();
}
