//use regex::bytes::Regex;
use regex::Regex;
use rexpect::error::Error as RexpectError;
use rexpect::session::{spawn as rexpect_spawn, PtySession};
use std::io::Write;

/// Strip a string until the 'alternate screen exit' escape code, and return the slice containing
/// the remaining string.
fn strip_until_alternate_screen_exit(text: &str) -> &str {
    // \u{1b}[?1049l - exit alternate screen
    let ptn = Regex::new(r"\x1b\[\?1049l").unwrap();
    if let Some(m) = ptn.find(text) {
        &text[m.end()..]
    } else {
        text
    }
}

/// Initialize the app and wait until it has entered the alternate screen, and return a handle to
/// the rexpect PtySession, which is ready for input.
fn run_app() -> PtySession {
    let mut proc = rexpect_spawn(
        // explicitly pass empty history file so we don't get first run prompt
        &format!("{} --history-file ''", env!("CARGO_BIN_EXE_tere")),
        Some(1_000),
    )
    .expect("error spawning process");

    // \u{1b}[?1049h - enter alternate screen
    proc.exp_string("\x1b[?1049h").unwrap();
    proc
}

#[test]
fn output_on_exit_without_cd() -> Result<(), RexpectError> {
    let mut proc = run_app();

    proc.send_control('c')?;
    proc.writer.flush()?;

    let output = proc.exp_eof()?;
    let output = strip_until_alternate_screen_exit(&output);

    assert_eq!(output, "tere: Exited without changing folder\r\n");

    Ok(())
}
