/// This module contains functionality for checking if the app is being run for the first time.

use std::path::PathBuf;
use std::io::Stderr;
use std::convert::TryFrom;

use crate::settings::TereSettings;
use crate::ui::markup_render::{wrap_and_stylize, README_STR};

use crate::error::TereError;

const FIRST_RUN_MESSAGE_FMT: &str = "Welcome to `tere`!

It seems like you are running `tere` for the first time. To use `tere` for changing directories, you need to make a small addition to your shell configuration. For example, for Bash/Zsh, you have to put the following in your `.bashrc`/`.zshrc`:

{}

For other shells and additional details such as configuring `tere`, see the `README`.

Press `y` to confirm that you have updated your shell configuration, or any other key to cancel.
";

/// Determine whether the app is being run for the first time, and if so, prompt the user to
/// configure their shell. If the app has been run before, or the user responds affirmatively to
/// the prompt, return Ok, otherwise return an error.
pub fn check_first_run_with_prompt(
    settings: &TereSettings,
    window: &mut Stderr,
) -> Result<(), TereError> {
    let hist_file = &settings.history_file;

    // For now we use a bit of a heuristic to determine if the app is being run for the first time:
    // we assume that the app has been run before if the history file exists, or the user
    // explicitly requests no history file. (Or one more possibility is that the cache directory
    // doesn't exist. In this case, we assume that the user knows what they're doing, and don't
    // prompt either.)
    //
    // Earlier I also had the idea to write the current version of the app to a `version` file in
    // the cache folder, which would signify that the app has been run before, but for now the
    // history file is enough.
    if hist_file.is_none() // user passed empty history file
        || PathBuf::from(hist_file.as_ref().unwrap()).try_exists().unwrap_or(false) // history file exists
    {
        Ok(())
    } else {
        prompt_first_run(window)
    }
}

fn prompt_first_run(window: &mut Stderr) -> Result<(), TereError> {
    //TODO: move drawing stuff to separate module under UI? so we don't have to import all crossterm stuff here...
    use crossterm::{
        terminal, cursor, style, queue, execute,
        event::{read as read_event, Event, KeyCode, KeyEvent}
    };

    let mut draw = || -> Result<(), TereError> {
        execute!(
            window,
            terminal::Clear(terminal::ClearType::All),
        )?;

        let (w, h) = terminal::size()?;
        for (i, line) in get_formatted_first_run_message(w as usize)
            .iter()
            .enumerate()
            .take(h as usize)
        {
            queue!(
                window,
                cursor::MoveTo(0, u16::try_from(i).unwrap_or(u16::MAX)),
            )?;

            for fragment in line {
                queue!(
                    window,
                    style::PrintStyledContent(fragment.clone())
                )?;
            }
        }
        execute!(window)?;
        Ok(())
    };

    draw()?;

    loop {
        match read_event()? {
            Event::Key(KeyEvent { code: KeyCode::Char('y'), .. }) => return Ok(()),
            Event::Resize(_, _) => draw()?,
            _ => return Err(TereError::FirstRunPromptCancelled("Cancelled.".to_string())),
        }
    }
}

fn get_formatted_first_run_message(
    width: usize,
) -> Vec<Vec<crossterm::style::StyledContent<String>>> {
    let shell_snippet = README_STR
        .split_once("Put this in your `.bashrc` or `.zshrc`:\n\n```sh\n")
        .expect("Could not find shell snippet in README")
        .1;

    let shell_snippet = &shell_snippet[
        ..
        shell_snippet.find("\n```").expect("Could not find end of shell snippet in README")
    ];

    // can't use format!() with FIRST_RUN_MESSAGE_FMT as the, since macros run before name
    // resolution (see https://github.com/rust-lang/rust/issues/69133#issuecomment-585789037),
    // so have to manually do it (I don't want to pull in const_format for just this).
    let first_run_message = FIRST_RUN_MESSAGE_FMT.replacen("{}", shell_snippet, 1);
    wrap_and_stylize(&first_run_message, width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_snippet_found() {
        // This should panic if the shell snippet used in the first run message is missing or
        // incorrectly formatted in the README
        get_formatted_first_run_message(100);
    }
}
