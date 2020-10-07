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
            Some(Input::KeyDC) | Some(Input::Character('\x1B')) => break,
            Some(Input::Character(c)) => { main_win.addstr(&format!("{}", c)); },
            Some(input) => { main_win.addstr(&format!("{:?}", input)); },
            None => (),
        }
        main_win.refresh();
    }

    endwin();
}
