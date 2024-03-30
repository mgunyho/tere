#[cfg(unix)]
mod tests {

use regex::Regex;
use rexpect::error::Error as RexpectError;
use rexpect::session::{spawn_command, PtySession};
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

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

/// Initialize app with the given Command object and wait until it has entered the alternate screen.
/// Returns a handle to the rexpect PtySession, which is ready for input. Panics if initializing
/// the app fails.
fn run_app_with_cmd(cmd: Command) -> PtySession {
    let mut proc = spawn_command(cmd, Some(1_000)).expect("error spawning process");

    // \u{1b}[?1049h - enter alternate screen
    proc.exp_string("\x1b[?1049h").unwrap();
    proc
}

fn get_cmd() -> Command {
    Command::new(std::env::var("CARGO_BIN_EXE_tere").unwrap())
}

/// Initialize the app command with the history file explicitly set to empty, so that we don't get
/// the first run prompt
fn get_cmd_no_first_run_prompt() -> Command {
    let mut cmd = get_cmd();
    // NOTE: cannot directly chain this with get_cmd(), otherwise we get a mutable ref which we
    // can't move out of this function
    cmd.args(["--history-file", ""]);
    cmd
}

#[test]
fn basic_run() -> Result<(), RexpectError> {
    let mut cmd = get_cmd_no_first_run_prompt();
    let tmp = tempdir().expect("error creating temporary folder");
    cmd.current_dir(tmp.path())
        // note: have to set PWD for this to work...
        .env("PWD", tmp.path().as_os_str());

    let mut proc = run_app_with_cmd(cmd);
    // 0x1b == 0o33 == 27 escape
    proc.send("\x1b")?;
    proc.writer.flush()?;

    let output = proc.exp_eof()?;
    let output = strip_until_alternate_screen_exit(&output);
    assert_eq!(output, format!("{}\r\n", tmp.path().display()));

    Ok(())
}

#[test]
fn output_on_exit_without_cd() -> Result<(), RexpectError> {
    let mut proc = run_app_with_cmd(get_cmd_no_first_run_prompt());

    proc.send_control('c')?;
    proc.writer.flush()?;

    let output = proc.exp_eof()?;
    let output = strip_until_alternate_screen_exit(&output);

    assert_eq!(output, "tere: Exited without changing folder\r\n");

    Ok(())
}

#[test]
fn first_run_prompt_cancel() -> Result<(), RexpectError> {
    let mut cmd = get_cmd();

    // set the XDG_CACHE_HOME to point to a temporary folder so that we always get the first run prompt
    let tmp = tempdir().expect("error creating temporary folder");
    cmd.env("XDG_CACHE_HOME", tmp.path().as_os_str());

    let mut proc = run_app_with_cmd(cmd);
    proc.send("n")?;
    proc.writer.flush()?;
    let output = proc.exp_eof()?;

    let ptn = Regex::new("It seems like you are running.*for the first time").unwrap();
    // check that first run prompt message is there
    assert!(ptn.find(&output).is_some());

    // check that having pressed 'n' prints the expected message
    assert_eq!(strip_until_alternate_screen_exit(&output), "Cancelled.\r\n");

    Ok(())
}

#[test]
fn first_run_prompt_accept() -> Result<(), RexpectError> {
    let mut cmd = get_cmd();

    // set the XDG_CACHE_HOME to point to a temporary folder so that we always get the first run prompt
    let tmp = tempdir().expect("error creating temporary folder");
    cmd.env("XDG_CACHE_HOME", tmp.path().as_os_str())
        .current_dir(tmp.path())
        .env("PWD", tmp.path());

    let mut proc = run_app_with_cmd(cmd);
    proc.send("y")?;
    proc.writer.flush()?;

    // 0x1b = escape
    proc.send("\x1b")?;
    proc.writer.flush()?;

    let output = proc.exp_eof()?;

    let ptn = Regex::new("It seems like you are running.*for the first time").unwrap();
    // check that first run prompt message was printed
    assert!(ptn.find(&output).is_some());

    // check that having pressed 'y' and then esc proceeded to the normal operation
    // and the history file was created
    assert_eq!(
        strip_until_alternate_screen_exit(&output),
        format!("{}\r\n", tmp.path().display()),
    );

    assert!(tmp.path().join("tere").join("history.json").exists());

    Ok(())
}
}
