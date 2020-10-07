use pancurses::{initscr, endwin, noecho, Input};

fn main() {
    let window = initscr();

    window.printw("hëllö");
    window.refresh();
    noecho();
    loop {
        match window.getch() {
            Some(Input::KeyDC) | Some(Input::Character('\x1B')) => break,
            Some(Input::Character(c)) => { window.addstr(&format!("{}", c)); },
            Some(input) => { window.addstr(&format!("{:?}", input)); },
            None => (),
        }
    }

    endwin();
}
