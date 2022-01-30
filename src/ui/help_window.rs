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
pub fn get_formatted_help_text<'a>(w: u16) -> Vec<Vec<StyledContent<String>>> {
    let help_str = &README_STR[
        README_STR.find("## User guide").expect("Could not find user guide in README")
            ..
        README_STR.find("## Similar projects").expect("Could not find end of user guide in README")
    ];

    // Skip the table of keyboard shortcuts, we'll format it separately
    let (help_str, rest) = help_str
        .split_once("\n\n|")
        .expect("Could not find keyboard shortcuts table in readme");

    let rest = rest.split_once("\n\n")
        .expect("Could not find end of keyboard shortcuts table in readme")
        .1;

    // Add justified keyboard shortcuts table to help string
    let mut help_str = help_str.to_string();
    help_str.push_str(&"\n\n"); // add back newlines eaten by split_once
    help_str.push_str(&get_justified_keyboard_shortcuts_table());
    help_str.push_str(rest);

    // We need to get rid of the `<kbd>` tags before wrapping so it works correctly. We're going to
    // bold all words within backticks, so replace the tags with backticks as well.
    let help_str = help_str
        .replace("<kbd>", "`")
        .replace("</kbd>", "`");

    // apply text wrapping
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

            // Make items inside backticks bold. Assuming that back-ticked items are completely on
            // a single line.
            let styled = line
                .split('`')
                .fold((false, vec![]), |(bold, mut acc), word| {
                    let word = word.to_string();
                    acc.push(if bold { word.bold() } else { word.stylize() });
                    (!bold, acc)
                }).1;
            res.push(styled);
        }
    }
    res

    //res.iter().map(|line| vec![StyledContent::new(ContentStyle::new(), line.to_string())]).collect()

}


/// Apply justification to the table of keyboard shortcuts in the README and render it to a String
/// without the markup
pub fn get_justified_keyboard_shortcuts_table() -> String {
    let keyboard_shortcuts = README_STR
        .split_once("keyboard shortcuts:\n\n")
        .expect("Couldn't find table of keyboard shortcuts in README")
        .1;
    let keyboard_shortcuts = keyboard_shortcuts
        .split_once("\n\n")
        .expect("Couldn't find end of keyboard shortcuts table in README")
        .0;

    let first_column_width = keyboard_shortcuts.lines()
        .map(|line| line.split("|").skip(1).next().unwrap_or("").len())
        .max()
        .unwrap_or(10);

    let mut justified = String::new();

    for (i, line) in keyboard_shortcuts.lines().enumerate() {
        let cols: Vec<&str> = line.split("|").collect();
        // cols[0] is empty, because the lines start with '|'.
        let mut action = cols[1].trim().to_string();
        let mut shortcut = cols[2].trim().to_string();

        // skip markdown table formatting row
        if action.starts_with(":--") {
            continue
        }

        if i == 0 {
            // add backticks so that first line is bolded
            action = format!("`{}`", &action);
            shortcut = format!("`{}`", &shortcut);
        }

        justified.push_str(&action);

        // backticks will be removed, so add extra space for them
        let extra_len = action.chars().filter(|c| *c == '`').count();
        let padding = first_column_width + extra_len + 2 - action.len();
        justified.push_str(&" ".repeat(padding));
        // It's ok to add "\n" at the end of every line, because the split_once() above has
        // eaten too many newlines from the end anyway.
        justified.push_str(&shortcut);
        justified.push('\n');
    }

    // add extra newline at end
    justified.push('\n');

    justified
}
