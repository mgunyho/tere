/// Functions for rendering the help window

use std::borrow::Cow;
use crossterm::style::{ContentStyle, StyledContent, Stylize};
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

    // We need to get rid of the `<kbd>` tags before wrapping so it works correctly. We're going to
    // bold all words within backticks, so replace the tags with backticks as well.
    let help_str = help_str
        .replace("<kbd>", "`")
        .replace("</kbd>", "`");

    let mut help_str = textwrap::wrap(&help_str, w as usize);

    let mut res = vec![];
    for line in help_str.drain(..) {
        if line.starts_with("#") {
            let styled = line
                .replace("# ", "")
                .replace("#", "")
                .to_string()
                .bold();
            res.push(vec![styled]);
        } else {
            //TODO: table formatting for keyboard shortcuts
            res.push(vec![line.to_string().stylize()]);
        }
    }
    res

    //res.iter().map(|line| vec![StyledContent::new(ContentStyle::new(), line.to_string())]).collect()

}
