/// This module contains some functions to do some extremely basic rendering of markdown to
/// `StyledContent` compatible with crossterm. Because the rendering is so simple, we are doing it
/// ourselves instead of a heavy, full-fledged markdown crate, at least for now.

use crossterm::style::{StyledContent, Stylize};

pub const README_STR: &str = include_str!("../../README.md");

/// Strip the markup (markdown) from some text and wrap it to a given width. The result is a
/// vector of lines, where each line is a vector of styled elements.
pub fn wrap_and_stylize(text: &str, width: usize) -> Vec<Vec<StyledContent<String>>> {
    // Strip out markup and extract the locations where we need to toggle bold on/off.
    let (mut text, bold_toggle_locs) = strip_markup_and_extract_bold_positions(text);

    // apply text wrapping
    textwrap::fill_inplace(&mut text, width);

    // Apply bold at the toggle locations. Have to be mut so that the drain/filter below works.
    let mut result = stylize_wrapped_lines(text.split('\n').collect(), bold_toggle_locs);

    // remove empty items (e.g. if the line started with a bold item).
    result
        .drain(..)
        .map(|mut line| {
            line.drain(..)
                .filter(|item| !item.content().is_empty())
                .collect()
        })
        .collect()
}

/// Return a version of `text`, where all markup has been strippeed, and also return a vector of
/// indices into the returned string where bold should toggle.
fn strip_markup_and_extract_bold_positions(text: &str) -> (String, Vec<usize>) {
    let mut bold_toggle_locs: Vec<usize> = vec![];
    let mut help_string_no_markup = String::new();
    let mut prev_char: Option<char> = None;
    let mut parsing_heading = false;
    let mut counter = 0;
    for c in text.chars() {
        if c == '#' {
            if !parsing_heading {
                parsing_heading = true;
                bold_toggle_locs.push(counter);
            }
        } else if c == ' ' && parsing_heading && prev_char == Some('#') {
            // skip space after hashes that indicate heading
        } else if c == '\n' && parsing_heading {
            bold_toggle_locs.push(counter);
            parsing_heading = false;
            counter += 1;
            help_string_no_markup.push(c);
        } else if c == '`' {
            bold_toggle_locs.push(counter);
        } else {
            counter += 1;
            help_string_no_markup.push(c);
        }
        prev_char = Some(c);
    }

    (help_string_no_markup, bold_toggle_locs)
}

/// Apply stylization to the text. Toggle bold at the positions indicated by `bold_toggle_locs`.
fn stylize_wrapped_lines<S>(
    lines: Vec<S>,
    bold_toggle_locs: Vec<usize>,
) -> Vec<Vec<StyledContent<String>>>
where
    S: AsRef<str>,
{
    let mut counter = 0;
    let mut bold_toggle_locs = bold_toggle_locs.iter();
    let mut next_toggle_loc = bold_toggle_locs.next();
    let mut res = vec![];
    let mut bold = false;

    for line in lines {
        let mut line_chunks = vec![];
        let mut cur_chunk = String::new();

        for c in line.as_ref().chars() {
            if Some(&counter) == next_toggle_loc {
                line_chunks.push(if bold {
                    cur_chunk.bold()
                } else {
                    cur_chunk.stylize()
                });
                bold = !bold;
                next_toggle_loc = bold_toggle_locs.next();
                cur_chunk = String::new();
            }
            cur_chunk.push(c);
            counter += 1;
        }

        if !cur_chunk.is_empty() {
            line_chunks.push(if bold {
                cur_chunk.bold()
            } else {
                cur_chunk.stylize()
            });
        }

        // always turn off bold at the end of the line
        if bold {
            bold = false;
            next_toggle_loc = bold_toggle_locs.next();
        }

        res.push(line_chunks);

        // increment counter for newline
        counter += 1;
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markup() {
        let input = "## foo bar\n\nlorem ipsum `dolor` sit amet";
        let (output, locs) = strip_markup_and_extract_bold_positions(input);
        assert_eq!(output, "foo bar\n\nlorem ipsum dolor sit amet");
        assert_eq!(locs, vec![0, 7, 21, 26]);
    }

    #[test]
    fn test_strip_markup2() {
        let input = "## foo bar\n\nlorem ipsum `dolor\nsit` amet";
        let (output, locs) = strip_markup_and_extract_bold_positions(input);
        assert_eq!(output, "foo bar\n\nlorem ipsum dolor\nsit amet");
        assert_eq!(locs, vec![0, 7, 21, 30]);
    }

    #[test]
    fn test_stylize_wrapped_lines() {
        let lines = vec!["foo bar", "", "lorem ipsum dolor sit amet"];
        let stylized = stylize_wrapped_lines(lines, vec![0, 7, 21, 26]);

        assert_eq!(
            stylized[0],
            vec!["".to_string().stylize(), "foo bar".to_string().bold()]
        );
        assert_eq!(stylized[1], vec![]);
        assert_eq!(stylized[2][0], "lorem ipsum ".to_string().stylize());
        assert_eq!(stylized[2][1], "dolor".to_string().bold());
        assert_eq!(stylized[2][2], " sit amet".to_string().stylize());
    }

    #[test]
    fn test_stylize_wrapped_lines2() {
        let input = "## foo bar\n\nlorem ipsum `dolor` sit amet";
        let (lines, locs) = strip_markup_and_extract_bold_positions(input);
        let stylized = stylize_wrapped_lines(lines.split('\n').collect(), locs);

        assert_eq!(
            stylized[0],
            vec!["".to_string().stylize(), "foo bar".to_string().bold()]
        );
        assert_eq!(stylized[1], vec![]);
        assert_eq!(stylized[2][0], "lorem ipsum ".to_string().stylize());
        assert_eq!(stylized[2][1], "dolor".to_string().bold());
        assert_eq!(stylized[2][2], " sit amet".to_string().stylize());
    }

    #[test]
    fn test_wrap_and_stylize() {
        // test case where textwrap adds an extra newline in the middle of a bold section of text,
        // where the newline does *not* replace whitespace (i.e. after a special character such as
        // '/')

        let input = "## foo bar\n\nlorem ipsum `dolor/sit` amet";
        let stylized = wrap_and_stylize(input, 18);

        assert_eq!(stylized[0], vec!["foo bar".to_string().bold()]);
        assert_eq!(stylized[1], vec![]);
        assert_eq!(stylized[2][0], "lorem ipsum".to_string().stylize());
        assert_eq!(stylized[3][0], "dolor/sit".to_string().bold());
        assert_eq!(stylized[3][1], " amet".to_string().stylize());
    }
}
