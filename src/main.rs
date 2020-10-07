use pancurses::{initscr, endwin, noecho, Input};

const HEADER_SIZE: i32 = 1;

fn main() {
    let window = initscr();

    let header_win = window.subwin(HEADER_SIZE, 0, 0, 0).unwrap();
    let main_win = window.subwin(window.get_max_y() - HEADER_SIZE, 0, HEADER_SIZE, 0).unwrap();

    header_win.addstr("this is the header");

    main_win.printw("hëllö");

    header_win.refresh();
    main_win.refresh();

    noecho();
    loop {
        match window.getch() {
            Some(Input::Character('\x1B')) => {
                // Either ESC or ALT+key. If it's ESC, the next getch will be
                // err. If it's ALT+key, next getch will contain the key
                window.nodelay(true);
                match window.getch() {
                    Some(Input::Character(c)) => { main_win.addstr(&format!("ALT+{}", c)); },
                    _ => { break; },
                }
                window.nodelay(false);
            }
            Some(Input::KeyDC) => break,
            Some(Input::Character(c)) => { main_win.addstr(&format!("{}", c)); },
            Some(Input::KeyResize) => (),
            Some(input) => { main_win.addstr(&format!("{:?}", input)); },
            None => (),
        }
        main_win.refresh();
    }

    endwin();
}
