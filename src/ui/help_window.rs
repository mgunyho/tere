/// Functions for rendering the help window

use std::borrow::Cow;
use crossterm::style::{ContentStyle, StyledContent};
use textwrap;

const README_STR: &str = include_str!("../../README.md");

/// Word-wrap the help string to be displayed in the help window, and apply correct formatting
/// (such as bolding) using crossterm::style.
///
/// Returns a vector of vectors, where the outer vector represents lines, and the inner vector
/// contains either a single string for the whole line, or multiple strings, if the style varies
/// within the line.
pub fn get_formatted_help_text<'a>(w: u16, h: u16) -> Vec<Vec<StyledContent<String>>> {
    let help_str = &README_STR[
        README_STR.find("## User guide").expect("Could not find user guide in README")
            ..
        README_STR.find("## Similar projects").expect("Could not find end of user guide in README")
    ];

    //TODO: remove <kbd> etc
    //TODO: table formatting
    let res = textwrap::wrap(help_str, w as usize);
    res.iter().map(|line| vec![StyledContent::new(ContentStyle::new(), line.to_string())]).collect()

    //TODO: format things with bold etc
    //StyledContent::new(ContentStyle::new(), res)
}
