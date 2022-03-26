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
use dirs::home_dir;
use unicode_segmentation::UnicodeSegmentation;

use clap::{App, Arg, ArgMatches};

const HEADER_SIZE: u16 = 1;
const INFO_WIN_SIZE: u16 = 1;
const FOOTER_SIZE: u16 = 1;

//TODO: rustfmt
//TODO: clippy

mod app_state;
use app_state::{TereAppState, CaseSensitiveMode, GapSearchMode};

mod ui;
use ui::help_window::get_formatted_help_text;

mod error;
use error::TereError;

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
        ret.info_message(format!("{} {} - Press '?' to view help or Esc to exit.",
                                 env!("CARGO_PKG_NAME"),
                                 env!("CARGO_PKG_VERSION")).as_str())?;
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

        extra_msg.push_str(&format!("{} - ", self.app_state.settings.gap_search_mode));
        extra_msg.push_str(&format!("{} - ", self.app_state.settings.case_sensitive));

        queue!(
            win,
            cursor::MoveTo(0, footer_win_row),
            style::SetAttribute(Attribute::Reset),
            //TODO: prevent line wrap here
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
                               self.app_state.num_visible_items()));
        }
        execute!(
            win,
            //TODO: this subtraction can overflow if the window is extremely narrow
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

        //TODO: make customizable...
        let highlight_fg = style::Color::Black;
        let highlight_bg = style::Color::Grey;
        let matching_letter_bg = style::Color::DarkGrey;
        let symlink_color = style::Color::Cyan;

        let (item, is_dir, is_symlink) = self.app_state.get_item_at_cursor_pos(row.into()).map_or(
            // Draw empty text at the rows that are outside the listing buffer.
            ("".to_string(), false, false),
            |itm| (itm.file_name_checked(), itm.is_dir(), itm.is_symlink)
        );

        let attr = if is_dir {
            Attribute::Bold
        } else {
            Attribute::Dim
        };

        queue!(
            self.window,
            cursor::MoveTo(0, row_abs),
            style::SetAttribute(Attribute::Reset),
            style::ResetColor,
            style::SetAttribute(attr),
        )?;

        if is_symlink {
            queue!(self.window, style::SetForegroundColor(symlink_color))?;
        }

        let idx = self.app_state.cursor_pos_to_visible_item_index(row.into());
        if self.app_state.is_searching()
            && self.app_state.visible_match_indices().contains(&idx) {
            // All *byte offsets* that should be underlined
            let underline_locs: Vec<usize> = self.app_state
                .get_match_locations_at_cursor_pos(row as u32)
                .unwrap_or(&vec![])
                .iter()
                .map(|(start, end)| (*start..*end).collect::<Vec<usize>>())
                .flatten()
                .collect();

            // Use unicode_segmentation to find out the grapheme clusters corresponding to the
            // above byte offsets, and determine whether they should be underlined.
            let letters_underlining: Vec<(&str, bool)> = UnicodeSegmentation::grapheme_indices(item.as_str(), true)
                // this contains() could probably be optimized, but shouldn't be too bad.
                .map(|(i, c)| (c, underline_locs.contains(&i)))
                .collect();

            for (c, underline) in letters_underlining {

                let (underline, fg, bg)  = match (underline, highlight) {
                    (true,      _) => (
                        Attribute::Underlined,
                        style::Color::Reset,
                        matching_letter_bg,
                        ),
                    (false,  true) => (
                        Attribute::NoUnderline,
                        highlight_fg,
                        highlight_bg,
                        ),
                    (false, false) => (
                        Attribute::NoUnderline,
                        if is_symlink { symlink_color } else { style::Color::Reset },
                        style::Color::Reset,
                        ),
                };

                queue!(
                    self.window,
                    style::SetAttribute(underline),
                    style::SetBackgroundColor(bg),
                    style::SetForegroundColor(fg),
                    style::Print(c.to_string()),
                )?;

            }

            // color the rest of the line if applicable
            if highlight {
                queue!(
                    self.window,
                    style::SetBackgroundColor(highlight_bg),
                    terminal::Clear(terminal::ClearType::UntilNewLine),
                )?;
            }

            queue!(
                self.window,
                style::SetBackgroundColor(style::Color::Reset),
            )?;

        } else {
            if highlight {
                // figure out how much padding we need after the item
                let item_size = UnicodeSegmentation::graphemes(item.as_str(), true).count();

                queue!(
                    self.window,
                    style::SetBackgroundColor(highlight_bg),
                    //NOTE: not using symlink_color here because cyan looks bad on grey background
                    style::SetForegroundColor(highlight_fg),
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
            terminal::Clear(terminal::ClearType::UntilNewLine),
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

        // are there any matches?
        let any_matches = self.app_state.num_matching_items() > 0;
        let any_visible_items = self.app_state.num_visible_items() > 0;
        let is_search = self.app_state.is_searching();

        // Draw entries. No need to clear the whole main window, because draw_main_window_row takes
        // care of clearing each row when applicable.
        for row in 0..max_y {
            // highlight the current row under the cursor when applicable
            let highlight = self.app_state.cursor_pos == (row as u32)
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
        let old_cursor_pos = self.app_state.cursor_pos;
        let old_scroll_pos = self.app_state.scroll_pos;

        self.app_state.move_cursor(amount, wrap);

        if self.app_state.scroll_pos != old_scroll_pos {
            // redraw_main_window takes care of (un)highlighting the cursor row
            // and refreshing
            self.redraw_main_window()?;
        } else {
            self.unhighlight_row(u16::try_from(old_cursor_pos).unwrap_or(u16::MAX))?;
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

    pub fn update_main_window_dimensions(&mut self) -> CTResult<()> {
        let (w, h) = main_window_size()?;
        let (w, h) = (w as u32, h as u32);
        self.app_state.update_main_window_dimensions(w, h);
        Ok(())
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
                self.app_state.num_visible_items() as u32
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

    fn cycle_gap_search_mode(&mut self) -> CTResult<()> {
        self.app_state.settings.gap_search_mode = match self.app_state.settings.gap_search_mode {
            GapSearchMode::GapSearchFromStart => GapSearchMode::NoGapSearch,
            GapSearchMode::NoGapSearch => GapSearchMode::GapSearchAnywere,
            GapSearchMode::GapSearchAnywere => GapSearchMode::GapSearchFromStart,
        };
        //TODO: do the other stuff that self.on_search_char_does, notably, change dir if only one match. or should it?
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

                        KeyCode::Char('?') => {
                            self.help_view_loop()?;
                        }

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

                        KeyCode::Char('f') if k.modifiers == CONTROL => {
                            self.cycle_gap_search_mode()?;
                        }

                        KeyCode::Char(c) => self.on_search_char(c)?,

                        KeyCode::Backspace => self.erase_search_char()?,

                        _ => self.info_message(&format!("{:?}", k))?,
                    }
                },

                Event::Resize(_, _) => {
                    self.update_main_window_dimensions()?;
                    self.redraw_all_windows()?;
                }

                //TODO don't show this in release
                e => self.info_message(&format!("{:?}", e))?,
            }
        }

        self.app_state.on_exit()
    }

    fn help_view_loop(&mut self) -> CTResult<()> {
        self.info_message("Use ↓/↑ or j/k to scroll. Press Esc to exit help.")?;

        // clear main window only once here in the beginning, otherwise it causes flashing/blinking.
        self.queue_clear_main_window()?;

        // We don't need the help view scroll state anywhere else, so not worth it to put in
        // app_state, just keep it here.
        let mut help_view_scroll: usize = 0;

        self.draw_help_view(help_view_scroll)?;

        loop {
            match read_event()? {
                Event::Key(k) => {
                    match k.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            self.info_message("")?;
                            return self.redraw_all_windows();
                        },

                        KeyCode::Down | KeyCode::Char('j') => {
                            help_view_scroll += 1;
                            self.draw_help_view(help_view_scroll)?;
                        }

                        KeyCode::Up | KeyCode::Char('k') => {
                            help_view_scroll = help_view_scroll.checked_sub(1).unwrap_or(0);
                            self.draw_help_view(help_view_scroll)?;
                        }

                        _ => {},
                    }
                }

                Event::Resize(_, _) => {

                    self.update_main_window_dimensions()?;
                    // Redraw all windows except for main window
                    self.redraw_header()?;
                    self.redraw_info_window()?;
                    self.redraw_footer()?;
                    self.draw_help_view(help_view_scroll)?;
                }

                _ => {},
            }
        }
    }

    fn draw_help_view(&mut self, scroll: usize) -> CTResult<()> {

        queue!(
            self.window,
            cursor::MoveTo(0, HEADER_SIZE),
            style::SetAttribute(Attribute::Reset),
            style::ResetColor,
        )?;

        let (w, h) = main_window_size()?;
        let help_text = get_formatted_help_text(w);
        for (i, line) in help_text.iter()
                .skip(scroll)
                .chain(vec![vec![]].iter().cycle()) // add empty lines at the end
                .take(h as usize)
                .enumerate()
            {
            // Set up cursor position
            queue!(
                self.window,
                // have to do MoveToColumn(0) manually because we're in raw mode
                cursor::MoveToColumn(0),
                // don't print newline before first line
                style::Print(if i == 0 { "" } else { "\n"}),
            )?;

            // Print the fragments (which can have different styles)
            for fragment in line {
                queue!(
                    self.window,
                    style::PrintStyledContent(fragment.clone()),
                )?;
            }

            // Clear the rest of the row
            queue!(
                self.window,
                terminal::Clear(terminal::ClearType::UntilNewLine),
            )?;
        }

        execute!(self.window)?;

        Ok(())
    }
}

macro_rules! case_sensitive_template {
    ($x:tt, $y:tt) => {
        format!("This overrides the --{} and --{} options. You can also change the case sensitivity mode while the program is running with the keyboard shortcut ALT+C.", $x, $y)
    }
}

macro_rules! gap_search_mode_template {
    ($x:tt, $y:tt) => {
        format!("This overrides the --{} and --{} options. You can also change the search mode while the program is running with the keyboard shortcut CTRL+F.", $x, $y)
    }
}

fn main() -> Result<(), TereError> {

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
        .arg(Arg::new("gap-search")
             .long("gap-search")
             .short('g')
             .help("Match the search from the beginning, but allow gaps (default)")
             .long_help(format!("When searching, match items that start with the same character as the search query, but allow gaps between the search characters. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"dt\" would match \"DeskTop\" and \"DocumenTs\" but not \"downloads\", and searching for \"es\" would match none of the above. This is the default behavior.\n\n{}", gap_search_mode_template!("gap-search", "no-gap-search")).as_str())
             .overrides_with_all(&["gap-search", "gap-search-anywhere", "no-gap-search"])
             )
        .arg(Arg::new("gap-search-anywhere")
             .long("gap-search-anywhere")
             .short('G')
             .help("Match the search anywhere, and allow gaps")
             .long_help(format!("When searching, allow the search characters to appear anywhere in a file/folder name, possibly with gaps between them. For example, searching for \"do\" would match \"DesktOp\", \"DOcuments\", and \"DOwnloads\", while searching for \"es\" would match \"dESktop\" and \"documEntS\", but not \"downloads\".\n\n{}",
                        gap_search_mode_template!("gap-search-from-start", "no-gap-search")).as_str())
             .overrides_with_all(&["gap-search-anywhere", "no-gap-search"])
             )
        .arg(Arg::new("no-gap-search")
             .long("no-gap-search")
             .short('n')
             .help("Match the search from the beginning, and do not allow gaps")
             .long_help(format!("Disable gap-search. Match only consecutive characters from the beginning of the search query. For example, searching for \"do\" would match \"DOcuments\" and \"DOwnloads\", but not \"desktop\".\n\n{}", gap_search_mode_template!("gap-search", "gap-search-from-start")).as_str())
             .overrides_with("no-gap-search")
             )
        //TODO: if somebody wants this: '-N', '--no-gap-search-anywhere - don't allow gaps, but can start anywhere. maybe have to come up with a better long name.
        .arg(Arg::new("autocd-timeout")
             .long("autocd-timeout")
             .help("Timeout for auto-cd when there's only one match, in milliseconds. Use 'off' to disable auto-cd.")
             .long_help("If the current search matches only one folder, automatically change to it after this many milliseconds. If the value is 'off', automatic cding is disabled, and you have to manually enter the folder. Setting the timeout to zero is not recommended, because it makes navigation confusing.")
             .default_value("200")
             .value_name("TIMEOUT or 'off'")
             .overrides_with("autocd-timeout")
            )
        .arg(Arg::new("history-file")
             .long("history-file")
             .help("Save history to this file. Set to empty to disable.")
             .long_help("Save a history of visited folders in this file in JSON format. Set to empty to disable saving history. If not provided, defaults to '$CACHE_DIR/tere/history.json', where $CACHE_DIR is the cache directory, i.e. $XDG_CACHE_HOME or ~/.cache.")
             .takes_value(true)
             .value_name("FILE or ''")
            )
        .try_get_matches()
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
                e => return Err(e),
            }
        }
        Ok(path) => path
    };

    // No error, print cwd, as returned by the app state
    println!("{}", final_path.display());

    Ok(())
}
