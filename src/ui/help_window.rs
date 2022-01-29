/// Functions for rendering the help window

use std::borrow::Cow;
use crossterm::style::{ContentStyle, StyledContent};
use textwrap;

const README_STR: &str = include_str!("../../README.md");

pub fn get_formatted_help_text<'a>(w: u16, h: u16) -> Vec<Cow<'a, str>> {
    let res = &README_STR[
        README_STR.find("## User guide").expect("Could not find user guide in README")
            ..
        README_STR.find("## Similar projects").expect("Could not find end of user guide in README")
    ];

    //TODO: remove <kbd> etc
    //TODO: table formatting
    textwrap::wrap(res, w as usize)

    //TODO: format things with bold etc
    //StyledContent::new(ContentStyle::new(), res)
}
