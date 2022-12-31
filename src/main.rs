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

mod first_run_check;
use first_run_check::check_first_run_with_prompt;

mod ui;
use ui::TereTui;

mod error;
use error::TereError;

mod panic_guard;
use panic_guard::GuardWithHook;

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


    //TODO: should this alternate screen etc initialization (and teardown) be done by the UI?
    //Now the mouse capture enabling (which is kind of similar) is handled there.
    execute!(std::io::stderr(), terminal::EnterAlternateScreen)?;
    let res: Result<std::path::PathBuf, TereError> = {
        // We can use unwrap here, the guards will ensure that the panic is handled correctly.
        let _guard = GuardWithHook::new(|| execute!(std::io::stderr(), terminal::LeaveAlternateScreen).unwrap());
        execute!(std::io::stderr(), cursor::Hide).unwrap();
        {
            let _guard = GuardWithHook::new(|| execute!(std::io::stderr(), cursor::Show).unwrap());
            terminal::enable_raw_mode().unwrap();
            {
                let _guard = GuardWithHook::new(|| terminal::disable_raw_mode().unwrap());

                // we are now inside the alternate screen, so collect all errors and attempt
                // to leave the alt screen in case of an error

                let mut stderr = std::io::stderr();

                stderr.flush().map_err(TereError::from)
                    .and_then(|_| TereSettings::parse_cli_args(&cli_args))
                    .and_then(|(settings, warnings)| { check_first_run_with_prompt(&settings, &mut stderr)?; Ok((settings, warnings)) })
                    .and_then(|(settings, warnings)| TereAppState::init(settings, &warnings))
                    .and_then(|state| TereTui::init(state, &mut stderr))
                    .and_then(|mut ui| ui.main_event_loop()) // actually run the app and return the final path
            }
        }
    };

    // Check if there was an error
    let final_path = match res {
        Err(err) => {
            match err {
                // Print pretty error message if the error was in arg parsing
                TereError::Clap(e) => e.exit(),

                TereError::ExitWithoutCd(msg) | TereError::FirstRunPromptCancelled(msg) => {
                    eprintln!("{}", msg);
                    std::process::exit(1);
                }

                // exit in case of any other error
                e => return Err(e),
            }
        }
        Ok(path) => path,
    };

    // No error, print cwd, as returned by the app state
    println!("{}", final_path.display());

    Ok(())
}
