use std::io::Write;
use crossterm::{
    execute,
    terminal,
    cursor,
};

//TODO: rustfmt
//TODO: clippy

mod cli_args;

mod settings;
use settings::TereSettings;

mod app_state;
use app_state::TereAppState;

mod installation_check;
use installation_check::check_first_run_with_prompt;

mod ui;
use ui::TereTui;

mod error;
use error::TereError;


fn main() -> Result<(), TereError> {

    let cli_args = cli_args::get_cli_args()
        .try_get_matches()
        .unwrap_or_else(|err| {
            // custom error handling: clap writes '--help' and '--version'
            // to stdout by default, but we want to print those to stderr
            // as well to not interfere with the intended behavior of tere
            eprint!("{}", err);
            std::process::exit(1);
        });

    let mut stderr = std::io::stderr();

    //TODO: should this alternate screen etc initialization (and teardown) be done by the UI?
    //Now the mouse capture enabling (which is kind of similar) is handled there.
    execute!(
        stderr,
        terminal::EnterAlternateScreen,
        cursor::Hide,
    )?;

    // we are now inside the alternate screen, so collect all errors and attempt
    // to leave the alt screen in case of an error

    let res: Result<std::path::PathBuf, TereError> = terminal::enable_raw_mode()
        .and_then(|_| stderr.flush()).map_err(TereError::from)
        .and_then(|_| TereSettings::parse_cli_args(&cli_args))
        .and_then(|(settings, warnings)| { check_first_run_with_prompt(&settings, &mut stderr)?; Ok((settings, warnings)) })
        .and_then(|(settings, warnings)| TereAppState::init(settings, &warnings))
        .and_then(|state| TereTui::init(state, &mut stderr))
        .and_then(|mut ui| ui.main_event_loop()); // actually run the app and return the final path

    // Always disable raw mode
    let raw_mode_success = terminal::disable_raw_mode().map_err(TereError::from);
    // this 'and' has to be in this order to keep the path if both results are ok.
    let res = raw_mode_success.and(res);

    execute!(
        stderr,
        terminal::LeaveAlternateScreen,
        cursor::Show,
        )?;

    // Check if there was an error
    let final_path = match res {
        Err(err) => {
            match err {
                // Print pretty error message if the error was in arg parsing
                TereError::Clap(e) => e.exit(),

                TereError::ExitWithoutCd(msg) => {
                    eprintln!("{}", msg);
                    std::process::exit(1);
                },

                // exit in case of any other error
                e => return Err(e),
            }
        }
        Ok(path) => path
    };

    // No error, print cwd, as returned by the app state
    println!("{}", final_path.display());

    Ok(())
}
