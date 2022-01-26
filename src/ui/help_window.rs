/// Functions for rendering the help window

use crossterm::style::{ContentStyle, StyledContent};

const README_STR: &str = include_str!("../../README.md");

pub fn get_formatted_help_text(w: u16, h: u16) -> StyledContent<String> {
    let mut res = String::new();
    //TODO: actual formatting
    //TODO: check out how tui-rs does line wrapping
    for line in README_STR.lines().skip(45).take(h as usize) {
        res.push_str(line);
        res.push_str("\n");
    }
    StyledContent::new(ContentStyle::new(), res)
}
