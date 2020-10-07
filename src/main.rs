use pancurses::{initscr, endwin, noecho, Input};

fn main() {
    let window = initscr();

    window.printw("hëllö");
    window.refresh();
    noecho();
    loop {
        match window.getch() {
            Some(Input::Character(c)) => { window.addch(c); },
            Some(Input::KeyDC) => break,
            Some(input) => { window.addstr(&format!("{:?}", input)); },
            None => (),
        }
    }

    endwin();
}
