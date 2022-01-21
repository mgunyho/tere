use std::convert::TryFrom;
use std::io::{Stderr, Write};
use crossterm::{
    execute, queue,
    terminal,
    cursor,
    style::{self, Stylize, Attribute},
    event::{
        read as read_event,
        Event,
        KeyCode,
        KeyModifiers,
    },
    Result as CTResult,
};
use home::home_dir;

use clap::{App, Arg, ArgMatches};

const HEADER_SIZE: u16 = 1;
const INFO_WIN_SIZE: u16 = 1;
const FOOTER_SIZE: u16 = 1;

//TODO: rustfmt
//TODO: clippy

mod app_state;
use app_state::{TereAppState, CaseSensitiveMode};

/// This struct groups together ncurses windows for the main content, header and
/// footer, and an application state object
struct TereTui<'a> {
    window: &'a Stderr,
    app_state: TereAppState,
}

// Dimensions (width, height) of main window
fn main_window_size() -> CTResult<(u16, u16)> {
    let (w, h) = terminal::size()?;
    Ok((w, h.checked_sub(HEADER_SIZE + INFO_WIN_SIZE + FOOTER_SIZE).unwrap_or(0)))
}

enum TereError {
    IoError(std::io::Error),
    ClapError(clap::Error),
}


impl From<std::io::Error> for TereError
{
    fn from(e: std::io::Error) -> Self { Self::IoError(e) }
}

impl From<clap::Error> for TereError {
    fn from(e: clap::Error) -> Self { Self::ClapError(e) }
}


impl<'a> TereTui<'a> {

    pub fn init(args: &ArgMatches, window: &'a mut Stderr) -> Result<Self, TereError> {
        let (w, h) = main_window_size()?;
        let state = TereAppState::init(
            args,
            // TODO: have to convert to u32 here. but correct solution would be to use u16 instead in app_state as well
            w.into(), h.into()
        )?; //.map_err(|e| Error::new(ErrorKind::Other, e))?;
        let mut ret = Self {
            window: window,
            app_state: state,
        };

        ret.update_header()?;
        ret.redraw_all_windows()?;
        Ok(ret)
    }

    /// Queue up a command to clear a given row (starting from 0). Must be executed/flushed
    /// separately.
    fn queue_clear_row(&mut self, row: u16) -> CTResult<()> {
        queue!(
            self.window,
            cursor::MoveTo(0, row),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )
    }

    pub fn redraw_header(&mut self) -> CTResult<()> {
        //TODO: what to do if window is narrower than path?
        // add "..." to beginning? or collapse folder names? make configurable?
        // at least, truncate towards the left instead of to the right

        // must use variable here b/c can't borrow 'self' twice in execute!() below
        let mut win = self.window;
        self.queue_clear_row(0)?;
        execute!(
            win,
            cursor::MoveTo(0, 0),
            style::SetAttribute(Attribute::Reset),
            style::Print(&self.app_state.header_msg.clone().bold().underlined()),
        )
    }

    pub fn update_header(&mut self) -> CTResult<()> {
        self.app_state.update_header();
        // TODO: consider removing redraw here... (is inconsistent with the rest of the 'update' functions)
        self.redraw_header()
    }

    pub fn redraw_info_window(&mut self) -> CTResult<()> {
        let (_, h) = crossterm::terminal::size()?;
        let info_win_row = h - FOOTER_SIZE - INFO_WIN_SIZE;

        self.queue_clear_row(info_win_row)?;
        let mut win = self.window;
        execute!(
            win,
            cursor::MoveTo(0, info_win_row),
            style::SetAttribute(Attribute::Reset),
            style::Print(&self.app_state.info_msg.clone().bold()),
        )
    }

    /// Set/update the current info message and redraw the info window
    pub fn info_message(&mut self, msg: &str) -> CTResult<()> {
        //TODO: add thread with timeout that will clear the info message after x seconds?
        self.app_state.info_msg = msg.to_string();
        self.redraw_info_window()
    }

    pub fn error_message(&mut self, msg: &str) -> CTResult<()> {
        //TODO: red color (also: make it configurable)
        let error_msg = format!("error: {}", &msg);
        self.info_message(&error_msg)
    }

    pub fn redraw_footer(&mut self) -> CTResult<()> {
        let (w, h) = crossterm::terminal::size()?;
        let footer_win_row = h - FOOTER_SIZE;
        self.queue_clear_row(footer_win_row)?;

        let mut win = self.window;
        let mut extra_msg = String::new();

        extra_msg.push_str(&format!("{} ", self.app_state.settings.case_sensitive));

        queue!(
            win,
            cursor::MoveTo(0, footer_win_row),
            style::SetAttribute(Attribute::Reset),
            style::Print(&format!("{}: {}",
                                  if self.app_state.settings.filter_search { "filter" } else { "search" },
                                  self.app_state.search_string()
                                  ).bold()),
        )?;

        let cursor_idx = self.app_state.cursor_pos_to_visible_item_index(self.app_state.cursor_pos);
        if self.app_state.is_searching() {
            let index_in_matches = self.app_state
                .visible_match_indices().iter()
                .position(|x| *x == cursor_idx)
                .unwrap_or(0);

            extra_msg.push_str(&format!("{} / {} / {}",
                               index_in_matches + 1,
                               self.app_state.num_matching_items(),
                               self.app_state.num_total_items()));
        } else {
            //TODO: show no. of files/folders separately? like 'n folders, n files'
            extra_msg.push_str(&format!("{} / {}",
                               cursor_idx + 1,
                               self.app_state.visible_items().len()));
        }
        execute!(
            win,
            cursor::MoveTo(w - extra_msg.len() as u16, footer_win_row),
            style::SetAttribute(Attribute::Reset),
            style::Print(extra_msg.bold()),
        )
    }

    fn draw_main_window_row(&mut self,
                            row: u16,
                            highlight: bool,
                            ) -> CTResult<()> {
        let row_abs = row  + HEADER_SIZE;
        let w: usize = main_window_size()?.0.into();

        let (item, bold, italic) = self.app_state.get_item_at_cursor_pos(row.into()).map_or(
            ("".to_string(), false, false),
            |itm| (itm.file_name_checked(), itm.is_dir(), itm.is_symlink)
        );
        let item_size = item.len();

        let attr = if bold {
            Attribute::Bold
        } else {
            Attribute::Dim
        };

        self.queue_clear_row(row_abs)?;

        queue!(
            self.window,
            cursor::MoveTo(0, row_abs),
            style::SetAttribute(Attribute::Reset),
            style::ResetColor,
            style::SetAttribute(attr),
        )?;

        if italic {
            queue!(self.window, style::SetAttribute(Attribute::Italic))?;
        }

        let idx = self.app_state.cursor_pos_to_visible_item_index(row.into());
        if self.app_state.is_searching()
            && self.app_state.visible_match_indices().contains(&idx) {
            // print matching part
            let n = self.app_state.search_string().len();
            let item_matching = item.get(..n).unwrap_or(&item);
            let item_not_matching = item.get(n..).unwrap_or("");
            queue!(
                self.window,
                style::SetAttribute(Attribute::Underlined),
                style::SetBackgroundColor(style::Color::DarkGrey),
                style::Print(item_matching.get(..w).unwrap_or(&item_matching)),
                style::SetAttribute(Attribute::NoUnderline),
                style::SetBackgroundColor(style::Color::Reset),
            )?;
            if highlight {
                queue!(
                    self.window,
                    style::SetBackgroundColor(style::Color::Grey),
                    style::SetForegroundColor(style::Color::Black),
                )?;
            }
            queue!(
                self.window,
                style::Print(item_not_matching.get(..w.checked_sub(n).unwrap_or(0)).unwrap_or(&item_not_matching)),
                style::Print(" ".repeat(w.checked_sub(item_size).unwrap_or(0))),
            )?;

        } else {
            if highlight {
                queue!(
                    self.window,
                    style::SetBackgroundColor(style::Color::Grey),
                    style::SetForegroundColor(style::Color::Black),
                    style::Print(item.get(..w).unwrap_or(&item)),
                    style::Print(" ".repeat(w.checked_sub(item_size).unwrap_or(0))),
                )?;
            } else {
                queue!(
                    self.window,
                    style::Print(item.get(..w).unwrap_or(&item)),
                )?;
            }
        }
        execute!(
            self.window,
            style::ResetColor,
            style::SetAttribute(Attribute::Reset),
        )

    }

    // redraw row 'row' (relative to the top of the main window) without highlighting
    pub fn unhighlight_row(&mut self, row: u16) -> CTResult<()> {
        self.draw_main_window_row(u16::try_from(row).unwrap_or(u16::MAX), false)
    }

    pub fn highlight_row(&mut self, row: u32) -> CTResult<()> { //TODO: change row to u16
        // Highlight the row `row` in the main window. Row 0 is the first row of
        // the main window
        self.draw_main_window_row(u16::try_from(row).unwrap_or(u16::MAX), true)
    }

    fn queue_clear_main_window(&mut self) -> CTResult<()> {
        let (_, h) = main_window_size()?;
        for row in HEADER_SIZE..h+HEADER_SIZE {
            self.queue_clear_row(row)?;
        }
        Ok(())
    }

    pub fn highlight_row_exclusive(&mut self, row: u32) -> CTResult<()> { //TODO: make row u16
        // Highlight the row `row` exclusively, and hide all other rows.
        self.queue_clear_main_window()?;
        self.highlight_row(row)?;
        Ok(())
    }

    pub fn redraw_main_window(&mut self) -> CTResult<()> {

        let (_, max_y) = main_window_size()?;
        let mut win = self.window;

        self.queue_clear_main_window()?;

        // are there any matches?
        let any_matches = self.app_state.num_matching_items() > 0;
        let any_visible_items = self.app_state.visible_items().len() > 0; //TODO: ~O(n) calculation of 'n'
        let is_search = self.app_state.is_searching();

        // draw entries
        for row in 0..max_y {
            // highlight the current row under the cursor when applicable
            let highlight = self.app_state.cursor_pos == row.into()
                && (!is_search || (any_matches || any_visible_items));
            self.draw_main_window_row(row, highlight)?;
        }

        win.flush()
    }

    fn redraw_all_windows(&mut self) -> CTResult<()> {
        self.redraw_header()?;
        self.redraw_info_window()?;
        self.redraw_footer()?;
        self.redraw_main_window()?;
        Ok(())
    }

    /// Update the app state by moving the cursor by the specified amount, and
    /// redraw the view as necessary.
    pub fn move_cursor(&mut self, amount: i32, wrap: bool) -> CTResult<()> {
        self.unhighlight_row(u16::try_from(self.app_state.cursor_pos).unwrap_or(u16::MAX))?;

        let old_scroll_pos = self.app_state.scroll_pos;

        self.app_state.move_cursor(amount, wrap);

        if self.app_state.scroll_pos != old_scroll_pos {
            // redraw_main_window takes care of highlighting the cursor row
            // and refreshing
            self.redraw_main_window()?;
        } else {
            self.highlight_row(self.app_state.cursor_pos)?;
        }
        Ok(())
    }

    pub fn change_dir(&mut self, path: &str) -> CTResult<()> {
        //TODO: if there are no visible items, don't do anything?
        match self.app_state.change_dir(path) {
            Err(e) => {
                if cfg!(debug_assertions) {
                    self.error_message(&format!("{:?}", e))?;
                } else {
                    self.error_message(&format!("{}", e))?;
                }
            },
            Ok(()) => {
                self.update_header()?;
                self.info_message("")?;
            }
        }
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    pub fn on_search_char(&mut self, c: char) -> CTResult<()> {
        self.app_state.advance_search(&c.to_string());
        let n_matches = self.app_state.num_matching_items();
        if n_matches == 1 {
            // There's only one match, highlight it and then change dir if applicable
            if let Some(timeout) = self.app_state.settings.autocd_timeout {
                self.highlight_row_exclusive(self.app_state.cursor_pos)?;

                std::thread::sleep(std::time::Duration::from_millis(timeout));

                // ignore keys that were pressed during sleep
                while crossterm::event::poll(std::time::Duration::from_secs(0))
                    .unwrap_or(false) {
                        read_event()?;
                    }

                self.change_dir("")?;
            }
        } else if n_matches == 0 {
            self.info_message(app_state::NO_MATCHES_MSG)?;
        } else {
            self.info_message("")?;
        }
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    pub fn erase_search_char(&mut self) -> CTResult<()> {
        self.app_state.erase_search_char();

        if self.app_state.num_matching_items() == 0 {
            self.info_message(app_state::NO_MATCHES_MSG)?;
        } else {
            self.info_message("")?;
        }

        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    pub fn on_resize(&mut self) -> CTResult<()> {

        let (w, h) = main_window_size()?;
        let (w, h) = (w as u32, h as u32);
        self.app_state.update_main_window_dimensions(w, h);

        self.redraw_all_windows()
    }

    pub fn on_arrow_key(&mut self, up: bool) -> CTResult<()> {
        let dir = if up { -1 } else { 1 };
        if self.app_state.is_searching() {
            //TODO: handle case where 'is_searching' but there are no matches - move cursor?
            self.app_state.move_cursor_to_adjacent_match(dir);
            self.redraw_main_window()?;
        } else {
            self.move_cursor(dir, true)?;
        }
        self.redraw_footer()
    }

    // When the 'page up' or 'page down' keys are pressed
    pub fn on_page_up_down(&mut self, up: bool) -> CTResult<()> {
        if !self.app_state.is_searching() {
            let (_, h) = main_window_size()?;
            let delta = ((h - 1) as i32) * if up { -1 } else { 1 };
            self.move_cursor(delta, false)?;
            self.redraw_footer()?;
        } //TODO: how to handle page up / page down while searching? jump to the next match below view?
        Ok(())
    }

    fn on_go_to_home(&mut self) -> CTResult<()> {
        if let Some(path) = home_dir() {
            if let Some(path) = path.to_str() {
                self.change_dir(path)?;
            }
        }
        Ok(())
    }

    fn on_go_to_root(&mut self) -> CTResult<()> {
        self.change_dir("/")
    }

    // on 'home' or 'end'
    fn on_home_end(&mut self, home: bool) -> CTResult<()> {
        if !self.app_state.is_searching() {
            let target = if home {
                0
            } else {
                self.app_state.visible_items().len() as u32
            };
            self.app_state.move_cursor_to(target);
            self.redraw_main_window()?;
        } // TODO: else jump to first/last match
        Ok(())
    }

    fn cycle_case_sensitive_mode(&mut self) -> CTResult<()> {
        self.app_state.settings.case_sensitive = match self.app_state.settings.case_sensitive {
            CaseSensitiveMode::IgnoreCase => CaseSensitiveMode::CaseSensitive,
            CaseSensitiveMode::CaseSensitive => CaseSensitiveMode::SmartCase,
            CaseSensitiveMode::SmartCase => CaseSensitiveMode::IgnoreCase,
        };
        self.app_state.advance_search("");
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    pub fn main_event_loop(&mut self) -> CTResult<()> {
        #[allow(non_snake_case)]
        let ALT = KeyModifiers::ALT;
        #[allow(non_snake_case)]
        let CONTROL = KeyModifiers::CONTROL;
        // root_win is the window created by initscr()
        loop {
            match read_event()? {
                Event::Key(k) => {
                    match k.code {
                        KeyCode::Right | KeyCode::Enter => self.change_dir("")?,
                        KeyCode::Left => self.change_dir("..")?, //TODO: use std::path::Component::ParentDir
                        KeyCode::Up if k.modifiers == ALT => {
                            self.change_dir("..")?;
                        },
                        KeyCode::Up => self.on_arrow_key(true)?,
                        KeyCode::Down if k.modifiers == ALT => {
                            self.change_dir("")?;
                        },
                        KeyCode::Down => self.on_arrow_key(false)?,

                        KeyCode::PageUp => self.on_page_up_down(true)?,
                        KeyCode::PageDown => self.on_page_up_down(false)?,

                        KeyCode::Home if k.modifiers == CONTROL => {
                            self.on_go_to_home()?;
                        }
                        KeyCode::Char('h') if k.modifiers == CONTROL | ALT => {
                            self.on_go_to_home()?;
                        }
                        KeyCode::Char('r') if k.modifiers == CONTROL => {
                            self.on_go_to_root()?;
                        }

                        KeyCode::Home => self.on_home_end(true)?,
                        KeyCode::End => self.on_home_end(false)?,

                        KeyCode::Esc => {
                            if self.app_state.is_searching() {
                                self.app_state.clear_search();
                                self.redraw_main_window()?;
                                self.redraw_footer()?;
                            } else {
                                break;
                            }
                        },

                        // alt + hjkl
                        KeyCode::Char('h') if k.modifiers == ALT => {
                            self.change_dir("..")?;
                        }
                        KeyCode::Char('j') if k.modifiers == ALT => {
                            self.on_arrow_key(false)?;
                        }
                        KeyCode::Char('k') if k.modifiers == ALT => {
                            self.on_arrow_key(true)?;
                        }
                        KeyCode::Char('l') if k.modifiers == ALT => {
                            self.change_dir("")?;
                        }

                        // other chars with modifiers
                        KeyCode::Char('q') if k.modifiers == ALT => {
                            break;
                        }
                        KeyCode::Char('u') if (k.modifiers == ALT || k.modifiers == CONTROL) => {
                            self.on_page_up_down(true)?;
                        }
                        KeyCode::Char('d') if (k.modifiers == ALT || k.modifiers == CONTROL) => {
                            self.on_page_up_down(false)?;
                        }
                        KeyCode::Char('g') if k.modifiers == ALT => {
                            // like vim 'gg'
                            self.on_home_end(true)?;
                        }
                        KeyCode::Char('G') if k.modifiers.contains(ALT) => {
                            self.on_home_end(false)?;
                        }

                        KeyCode::Char('c') if k.modifiers == ALT => {
                            self.cycle_case_sensitive_mode()?;
                        }

                        KeyCode::Char(c) => self.on_search_char(c)?,

                        KeyCode::Backspace => self.erase_search_char()?,

                        _ => self.info_message(&format!("{:?}", k))?,
                    }
                },

                Event::Resize(_, _) => self.on_resize()?,

                //TODO don't show this in release
                e => self.info_message(&format!("{:?}", e))?,
            }
        }

        Ok(())
    }
}

macro_rules! case_sensitive_template {
    // NOTE: long lines don't wrap in long_help message with clap 2.33 (see https://github.com/clap-rs/clap/issues/2445). should update clap to v3.
    ($x:tt, $y:tt) => {
        format!("This overrides the --{} and --{} options. You can also change the case sensitivity mode while the program is running with the keyboard shortcut ALT+C.", $x, $y)
    }
}

fn main() -> crossterm::Result<()> {

    let cli_args = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        //.author(env!("CARGO_PKG_AUTHORS")) // TODO: rest of these https://stackoverflow.com/a/27841363
        .global_setting(clap::AppSettings::DeriveDisplayOrder)
        .arg(Arg::new("filter-search")
             .long("filter-search")
             //.visible_alias("fs") //TODO: consider
             .short('f')
             .help("Show only items matching the search in listing")
             .long_help("Show only items matching the search in listing. This overrides the --no-filter-search option.")
             .overrides_with("filter-search")
            )
        .arg(Arg::new("no-filter-search")
             .long("no-filter-search")
             //.visible_alias("nfs") //TODO: consider
             .short('F')
             .help("Show all items in the listing even when searching (default)")
             .long_help("Show all items in the listing even when searching (default). This overrides the --filter-search option.")
             .overrides_with_all(&["filter-search", "no-filter-search"])
            )
        .arg(Arg::new("folders-only")
             .long("folders-only")
             //.visible_alias("fo") //TODO: consider
             .short('d')
             .help("Show only folders in the listing")
             .long_help("Show only folders (and symlinks pointing to folders) in the listing. This overrides the --no-folders-only option.")
             .overrides_with("folders-only")
             )
        .arg(Arg::new("no-folders-only")
             .long("no-folders-only")
             //.visible_alias("nfo") //TODO: consider
             .short('D')
             .help("Show files and folders in the listing (default)")
             .long_help("Show both files and folders in the listing. This is the default view mode. This overrides the --folders-only option.")
             .overrides_with_all(&["folders-only", "no-folders-only"])
             )
        .arg(Arg::new("case-sensitive")
             .long("case-sensitive")
             .short('s')  // same as ripgrep
             .help("Case sensitive search")
             .long_help(format!("Enable case-sensitive search.\n\n{}",
                        case_sensitive_template!("ignore-case", "smart-case")).as_str())
             .overrides_with_all(&["ignore-case", "smart-case", "case-sensitive"])
            )
        .arg(Arg::new("ignore-case")
             .long("ignore-case")
             .short('i') // same as ripgrep
             .help("Ignore case when searching")
             .long_help(format!("Enable case-insensitive search.\n\n{}",
                        case_sensitive_template!("case-sensitive", "smart-case")).as_str())
             .overrides_with_all(&["smart-case", "ignore-case"])
            )
        .arg(Arg::new("smart-case")
             .long("smart-case")
             .short('S') // same as ripgrep
             .help("Smart case search (default)")
             .long_help(format!("Enable smart-case search. If the search query contains only lowercase letters, search case insensitively. Otherwise search case sensitively. This is the default search mode.\n\n{}",
                        case_sensitive_template!("case-sensitive", "ignore-case")).as_str())
             .overrides_with("smart-case")
            )
        .arg(Arg::new("autocd-timeout")
             .long("autocd-timeout")
             .help("Timeout for auto-cd when there's only one match, in ms. Use 'off' to disable auto-cd.")
             .long_help("If the current search matches only one folder, automatically change to it after this many milliseconds. If the value is 'off', automatic cding is disabled, and you have to manually enter the folder. Setting the timeout to zero is not recommended, because it makes navigation confusing.")
             .default_value("200")
             .value_name("TIMEOUT or 'off'")
             .overrides_with("autocd-timeout")
            )
        .get_matches_safe()
        .unwrap_or_else(|err| {
            // custom error handling: clap writes '--help' and '--version'
            // to stdout by default, but we want to print those to stderr
            // as well to not interfere with the intended behavior of tere
            eprint!("{}", err);
            std::process::exit(1);
        });

    let mut stderr = std::io::stderr();

    execute!(
        stderr,
        terminal::EnterAlternateScreen,
        cursor::Hide,
    )?;

    // we are now inside the alternate screen, so collect all errors and attempt
    // to leave the alt screen in case of an error

    let res: Result<std::path::PathBuf, TereError> = terminal::enable_raw_mode()
        .and_then(|_| stderr.flush()).map_err(TereError::from)
        .and_then(|_| TereTui::init(&cli_args, &mut stderr))
        .and_then(|mut ui| {
            ui.main_event_loop().map_err(TereError::from)
                .map(|_| ui.app_state.current_path)
        });

    // Always disable raw mode
    let raw_mode_success = terminal::disable_raw_mode().map_err(TereError::from);
    // this 'and' has to be in this order to keep the path if both results are ok.
    let res = raw_mode_success.and(res);

    execute!(
        stderr,
        terminal::LeaveAlternateScreen,
        cursor::Show
        )?;

    // Check if there was an error
    let final_path = match res {
        Err(err) => {
            match err {
                // Print pretty error message if the error was in arg parsing
                TereError::ClapError(e) => e.exit(),

                // exit in case of any other error
                TereError::IoError(e) => return Err(e),
            }
        }
        Ok(path) => path
    };

    // No error, print cwd, as returned by the app state
    println!("{}", final_path.display());

    Ok(())
}
