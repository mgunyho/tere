pub mod help_window;
mod action;

use std::convert::TryFrom;
use std::io::{Stderr, Write};
use std::path::PathBuf;
use std::fmt::Write as _;

use crate::error::TereError;
use crate::app_state::{
    TereAppState,
    CaseSensitiveMode,
    GapSearchMode,
    NO_MATCHES_MSG,
};
use help_window::get_formatted_help_text;
pub use action::{Action, ActionContext};

use crossterm::{
    execute,
    queue,
    terminal,
    cursor,
    style::{self, Stylize, Attribute},
    event::{
        read as read_event,
        Event,
        KeyEvent,
        MouseEvent,
        MouseEventKind,
        MouseButton,
        KeyCode,
        KeyModifiers,
        EnableMouseCapture,
        DisableMouseCapture,
    },
    Result as CTResult,
};

use clap::ArgMatches;
use dirs::home_dir;
use unicode_segmentation::UnicodeSegmentation;

const HEADER_SIZE: usize = 1;
const INFO_WIN_SIZE: usize = 1;
const FOOTER_SIZE: usize = 1;

/// This struct is responsible for drawing an app state object to a stderr stream (confusingly
/// called 'window' for historical reasons) that the UI is written to. Currently it somewhat
/// conflates application logic with the UI.
pub struct TereTui<'a> {
    window: &'a Stderr,
    app_state: TereAppState,
}

/// Return the current terminal size as a pair of `(usize, usize)` instead of `(u16, 16)` as
/// is done by crossterm.
fn terminal_size_usize() -> CTResult<(usize, usize)> {
    let (w, h): (u16, u16) = terminal::size()?;
    Ok((w as usize, h as usize))
}

// Dimensions (width, height) of main window
fn main_window_size() -> CTResult<(usize, usize)> {
    let (w, h) = terminal_size_usize()?;
    Ok((
        w as usize,
        (h as usize).saturating_sub(HEADER_SIZE + INFO_WIN_SIZE + FOOTER_SIZE),
    ))
}

impl<'a> TereTui<'a> {
    pub fn init(args: &ArgMatches, window: &'a mut Stderr) -> Result<Self, TereError> {
        let (w, h) = main_window_size()?;
        let state = TereAppState::init(args, w, h)?;
        let mut ret = Self {
            window,
            app_state: state,
        };

        if ret.app_state.settings().mouse_enabled {
            execute!(ret.window, EnableMouseCapture)?;
        }

        ret.update_header()?;
        ret.redraw_all_windows()?;
        ret.info_message(
            format!(
                "{} {} - Type something to search, press '?' to view help or Esc to exit.",
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            )
            .as_str(),
        )?;
        Ok(ret)
    }

    /// Get the current (logical) path to be printed on exit.
    pub fn current_path(&self) -> PathBuf {
        self.app_state.current_path.clone()
    }

    /// Queue up a command to clear a given row (starting from 0). Must be executed/flushed
    /// separately.
    fn queue_clear_row(&mut self, row: usize) -> CTResult<()> {
        queue!(
            self.window,
            cursor::MoveTo(0, u16::try_from(row).unwrap_or(u16::MAX)),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )
    }

    fn redraw_header(&mut self) -> CTResult<()> {
        //TODO: what to do if window is narrower than path?
        // add "..." to beginning? or collapse folder names? make configurable?
        // at least, truncate towards the left instead of to the right

        let (max_x, _) = main_window_size()?;

        let header_graphemes: Vec<String> =
            UnicodeSegmentation::graphemes(self.app_state.header_msg.as_str(), true)
                .map(String::from)
                .collect();
        let n_skip = header_graphemes.len().saturating_sub(max_x as usize);
        let header_msg = header_graphemes[n_skip..].join("");

        // must use variable here b/c can't borrow 'self' twice in execute!() below
        let mut win = self.window;
        self.queue_clear_row(0)?;
        execute!(
            win,
            cursor::MoveTo(0, 0),
            style::SetAttribute(Attribute::Reset),
            style::Print(&header_msg.bold().underlined()),
        )
    }

    fn update_header(&mut self) -> CTResult<()> {
        self.app_state.update_header();
        // TODO: consider removing redraw here... (is inconsistent with the rest of the 'update' functions)
        self.redraw_header()
    }

    fn redraw_info_window(&mut self) -> CTResult<()> {
        let (_, h) = terminal_size_usize()?;
        let info_win_row = h - FOOTER_SIZE - INFO_WIN_SIZE;

        self.queue_clear_row(info_win_row)?;
        let mut win = self.window;
        execute!(
            win,
            cursor::MoveTo(0, u16::try_from(info_win_row).unwrap_or(u16::MAX)),
            style::SetAttribute(Attribute::Reset),
            style::Print(&self.app_state.info_msg.clone().bold()),
        )
    }

    /// Set/update the current info message and redraw the info window
    fn info_message(&mut self, msg: &str) -> CTResult<()> {
        //TODO: add thread with timeout that will clear the info message after x seconds?
        self.app_state.info_msg = msg.to_string();
        self.redraw_info_window()
    }

    fn error_message(&mut self, msg: &str) -> CTResult<()> {
        //TODO: red color (also: make it configurable)
        let error_msg = format!("error: {}", &msg);
        self.info_message(&error_msg)
    }

    fn redraw_footer(&mut self) -> CTResult<()> {
        let (w, h) = terminal_size_usize()?;
        let footer_win_row = h - FOOTER_SIZE;
        self.queue_clear_row(footer_win_row)?;

        let mut win = self.window;
        let mut extra_msg = String::new();

        let _ = write!(extra_msg, "{} - ", self.app_state.settings().gap_search_mode);
        let _ = write!(extra_msg, "{} - ", self.app_state.settings().case_sensitive);

        let cursor_idx = self
            .app_state
            .cursor_pos_to_visible_item_index(self.app_state.cursor_pos);

        if self.app_state.is_searching() {
            let index_in_matches = self
                .app_state
                .visible_match_indices()
                .iter()
                .position(|x| *x == cursor_idx)
                .unwrap_or(0);

            let _ = write!(
                extra_msg,
                "{} / {} / {}",
                index_in_matches + 1,
                self.app_state.num_matching_items(),
                self.app_state.num_total_items()
            );
        } else {
            //TODO: show no. of files/folders separately? like 'n folders, n files'
            let _ = write!(
                extra_msg,
                "{} / {}",
                cursor_idx + 1,
                self.app_state.num_visible_items()
            );
        }

        // draw extra message first, so that it gets overwritten by the more important search query
        // if there is not enough space
        queue!(
            win,
            cursor::MoveTo(
                u16::try_from(w.saturating_sub(extra_msg.len())).unwrap_or(u16::MAX),
                u16::try_from(footer_win_row).unwrap_or(u16::MAX),
            ),
            style::SetAttribute(Attribute::Reset),
            style::Print(
                extra_msg
                    .chars()
                    .take(w as usize)
                    .collect::<String>()
                    .bold()
            ),
        )?;

        execute!(
            win,
            cursor::MoveTo(0, u16::try_from(footer_win_row).unwrap_or(u16::MAX)),
            style::SetAttribute(Attribute::Reset),
            //TODO: prevent line wrap here
            style::Print(
                &format!(
                    "{}: {}",
                    if self.app_state.settings().filter_search {
                        "filter"
                    } else {
                        "search"
                    },
                    self.app_state.search_string()
                )
                .bold()
            ),
        )
    }

    fn draw_main_window_row(&mut self, row: usize, highlight: bool) -> CTResult<()> {
        let row_abs = row + HEADER_SIZE;

        //TODO: make customizable...
        let highlight_fg = style::Color::Black;
        let highlight_bg = style::Color::Grey;
        let matching_letter_bg = style::Color::DarkGrey;
        let symlink_color = style::Color::Cyan;

        let item = self.app_state.get_item_at_cursor_pos(row);

        let text_attr = if item.map(|itm| itm.is_dir()).unwrap_or(false) {
            Attribute::Bold
        } else {
            Attribute::Dim
        };

        queue!(
            self.window,
            cursor::MoveTo(0, u16::try_from(row_abs).unwrap_or(u16::MAX)),
            style::SetAttribute(Attribute::Reset),
            style::ResetColor,
            style::SetAttribute(text_attr),
        )?;

        let idx = self.app_state.cursor_pos_to_visible_item_index(row);

        // All *byte offsets* that should be underlined
        let underline_locs = if self.app_state.is_searching()
            && self.app_state.visible_match_indices().contains(&idx)
        {
            self.app_state
                .get_match_locations_at_cursor_pos(row)
                .unwrap_or(&vec![])
                .iter()
                .flat_map(|(start, end)| (*start..*end).collect::<Vec<usize>>())
                .collect()
        } else {
            vec![]
        };

        let item_size = if let Some(item) = item {
            // we're actually drawing an item

            let symlink_target = &item.symlink_target;
            let is_symlink = symlink_target.is_some();
            let fname = item.file_name_checked();

            // Find out the grapheme clusters corresponding to the
            // above byte offsets, and determine whether they should be underlined.
            let letters_underlining: Vec<(&str, bool)> =
                UnicodeSegmentation::grapheme_indices(fname.as_str(), true)
                    // this contains() could probably be optimized, but shouldn't be too bad.
                    .map(|(i, c)| (c, underline_locs.contains(&i)))
                    .collect();

            // queue draw actions for each (non-)underlined segment
            for (c, underline) in &letters_underlining {

                let (underline, fg, bg) = match (underline, highlight) {
                    (true, _) => (
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
                        if is_symlink {
                            symlink_color
                        } else {
                            style::Color::Reset
                        },
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

            if let Some(target) = symlink_target {
                // target is OsStr, so use display() here. This is fine because we're not going to
                // use it for anything else.
                //TODO: different color for target?
                let target_text = format!(" -> {}", target.display());
                queue!(self.window, style::Print(&target_text))?;

                letters_underlining.len() + UnicodeSegmentation::graphemes(target_text.as_str(), true).count()
            } else {
                letters_underlining.len()
            }
        } else {
            0
        };

        // color the rest of the line if applicable
        let width: usize = main_window_size()?.0;
        if highlight && width > item_size {
            queue!(
                self.window,
                style::SetAttribute(Attribute::Reset), // so that the rest of the line isn't underlined
                style::SetBackgroundColor(highlight_bg),
                style::Print(" ".repeat(width.saturating_sub(item_size))),
            )?;
        }

        execute!(
            self.window,
            style::ResetColor,
            style::SetAttribute(Attribute::Reset),
            terminal::Clear(terminal::ClearType::UntilNewLine),
        )
    }

    // redraw row 'row' (relative to the top of the main window) without highlighting
    fn unhighlight_row(&mut self, row: usize) -> CTResult<()> {
        self.draw_main_window_row(row, false)
    }

    fn highlight_row(&mut self, row: usize) -> CTResult<()> {
        // Highlight the row `row` in the main window. Row 0 is the first row of
        // the main window
        self.draw_main_window_row(row, true)
    }

    fn queue_clear_main_window(&mut self) -> CTResult<()> {
        let (_, h) = main_window_size()?;
        for row in HEADER_SIZE..(h + HEADER_SIZE) {
            self.queue_clear_row(row)?;
        }
        Ok(())
    }

    fn highlight_row_exclusive(&mut self, row: usize) -> CTResult<()> {
        // Highlight the row `row` exclusively, and hide all other rows.
        self.queue_clear_main_window()?;
        self.highlight_row(row)?;
        Ok(())
    }

    fn redraw_main_window(&mut self) -> CTResult<()> {
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
            let highlight = self.app_state.cursor_pos == row
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
    fn move_cursor(&mut self, amount: isize, wrap: bool) -> CTResult<()> {
        let old_cursor_pos = self.app_state.cursor_pos;
        let old_scroll_pos = self.app_state.scroll_pos;

        self.app_state.move_cursor(amount, wrap);

        if self.app_state.scroll_pos != old_scroll_pos {
            // redraw_main_window takes care of (un)highlighting the cursor row
            // and refreshing
            self.redraw_main_window()?;
        } else {
            self.unhighlight_row(old_cursor_pos)?;
            self.highlight_row(self.app_state.cursor_pos)?;
        }
        Ok(())
    }

    /// Change the working directory. If successful, returns true. If unsuccessful, print an error
    /// message to the UI and return false.
    fn change_dir(&mut self, path: &str) -> CTResult<bool> {
        //TODO: if there are no visible items, don't do anything?
        let res = match self.app_state.change_dir(path) {
            Err(e) => {
                if cfg!(debug_assertions) {
                    self.error_message(&format!("{:?}", e))?;
                } else {
                    self.error_message(&format!("{}", e))?;
                }
                false
            }
            Ok(()) => {
                self.update_header()?;
                self.info_message("")?;
                true
            }
        };
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(res)
    }

    fn on_search_char(&mut self, c: char) -> CTResult<()> {
        self.app_state.advance_search(&c.to_string());
        let n_matches = self.app_state.num_matching_items();
        if n_matches == 1 {
            // There's only one match, highlight it and then change dir if applicable
            if let Some(timeout) = self.app_state.settings().autocd_timeout {
                self.highlight_row_exclusive(self.app_state.cursor_pos)?;

                std::thread::sleep(std::time::Duration::from_millis(timeout));

                // ignore keys that were pressed during sleep
                while crossterm::event::poll(std::time::Duration::from_secs(0)).unwrap_or(false) {
                    read_event()?;
                }

                self.change_dir("")?;
            }
        } else if n_matches == 0 {
            self.info_message(NO_MATCHES_MSG)?;
        } else {
            self.info_message("")?;
        }
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    fn erase_search_char(&mut self) -> CTResult<()> {
        self.app_state.erase_search_char();

        if self.app_state.num_matching_items() == 0 {
            self.info_message(NO_MATCHES_MSG)?;
        } else {
            self.info_message("")?;
        }

        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    fn on_clear_search(&mut self) -> CTResult<()> {
        self.app_state.clear_search();
        self.info_message("")?; // clear possible 'no matches' message
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    fn update_main_window_dimensions(&mut self) -> CTResult<()> {
        let (w, h) = main_window_size()?;
        self.app_state.update_main_window_dimensions(w, h);
        Ok(())
    }

    fn on_cursor_up_down(&mut self, up: bool) -> CTResult<()> {
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

    // When scroling up or down by a screenful (i.e. 'page up' or 'page down')
    fn on_cursor_up_down_screen(&mut self, up: bool) -> CTResult<()> {
        if !self.app_state.is_searching() {
            let (_, h) = main_window_size()?;
            let delta = ((h - 1) as isize) * if up { -1 } else { 1 };
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
        self.change_dir("/")?;
        Ok(())
    }

    // When moving the cursor to the top or bottom of the listing
    fn on_cursor_top_bottom(&mut self, top: bool) -> CTResult<()> {
        if !self.app_state.is_searching() {
            let target = if top {
                0
            } else {
                self.app_state.num_visible_items()
            };
            self.app_state.move_cursor_to(target);
            self.redraw_main_window()?;
        } // TODO: else jump to first/last match
        Ok(())
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) -> CTResult<()> {
        if event.row == 0 {
            //TODO: change to folder by clicking on path component in header
            return Ok(());
        }

        if let Some(entry) = self
            .app_state
            .get_item_at_cursor_pos((event.row - 1) as usize)
        {
            let fname = entry.file_name_checked();
            if event.kind == MouseEventKind::Up(MouseButton::Left) {
                self.change_dir(&fname)?;
            } else {
                self.app_state.move_cursor_to_filename(&fname);
                self.redraw_main_window()?;
            }
        }
        Ok(())
    }

    /// Things to do when one of the search modes (filter, gap search, case sensitivty) is changed.
    fn on_search_mode_change(&mut self) -> CTResult<()> {
        self.redraw_main_window()?;
        self.redraw_footer()?;
        Ok(())
    }

    fn toggle_filter_search_mode(&mut self) -> CTResult<()> {
        self.app_state.set_filter_search(!self.app_state.settings().filter_search);
        self.on_search_mode_change()
    }

    fn cycle_case_sensitive_mode(&mut self) -> CTResult<()> {
        self.app_state.set_case_sensitive(match self.app_state.settings().case_sensitive {
            CaseSensitiveMode::IgnoreCase => CaseSensitiveMode::CaseSensitive,
            CaseSensitiveMode::CaseSensitive => CaseSensitiveMode::SmartCase,
            CaseSensitiveMode::SmartCase => CaseSensitiveMode::IgnoreCase,
        });
        self.on_search_mode_change()
    }

    fn cycle_gap_search_mode(&mut self) -> CTResult<()> {
        self.app_state.set_gap_search_mode(match self.app_state.settings().gap_search_mode {
            GapSearchMode::GapSearchFromStart => GapSearchMode::NoGapSearch,
            GapSearchMode::NoGapSearch => GapSearchMode::GapSearchAnywere,
            GapSearchMode::GapSearchAnywere => GapSearchMode::GapSearchFromStart,
        });
        self.on_search_mode_change()
    }

    pub fn main_event_loop(&mut self) -> Result<(), TereError> {

        let loop_result = loop {
            match read_event()? {
                Event::Key(k) => {
                    let valid_ctx = if self.app_state.is_searching() {
                        ActionContext::Searching
                    } else {
                        ActionContext::NotSearching
                    };

                    let action = self
                        .app_state
                        .settings().keymap.get(&(k, valid_ctx))
                        // If no mapping is found with the currently applying context, look for a
                        // mapping that applies in any context
                        .or_else(|| self.app_state.settings().keymap.get(&(k, ActionContext::None)));

                    if let Some(action) = action {
                        match action {
                            Action::ChangeDir => { self.change_dir("")?; },
                            Action::ChangeDirParent => { self.change_dir("..")?; },
                            Action::ChangeDirHome => self.on_go_to_home()?,
                            Action::ChangeDirRoot => self.on_go_to_root()?,

                            Action::ChangeDirAndExit => {
                                if self.change_dir("")? {
                                    break Ok(());
                                }
                            },

                            Action::CursorUp => self.on_cursor_up_down(true)?,
                            Action::CursorDown => self.on_cursor_up_down(false)?,
                            Action::CursorUpScreen => self.on_cursor_up_down_screen(true)?,
                            Action::CursorDownScreen => self.on_cursor_up_down_screen(false)?,
                            Action::CursorTop => self.on_cursor_top_bottom(true)?,
                            Action::CursorBottom => self.on_cursor_top_bottom(false)?,

                            Action::EraseSearchChar => self.erase_search_char()?,

                            Action::ClearSearch => self.on_clear_search()?,

                            Action::ChangeFilterSearchMode => self.toggle_filter_search_mode()?,
                            Action::ChangeCaseSensitiveMode => self.cycle_case_sensitive_mode()?,
                            Action::ChangeGapSearchMode => self.cycle_gap_search_mode()?,

                            Action::RefreshListing => {
                                self.change_dir(".")?; //TODO: use 'current dir' instead of hardcoded '.' (?, see also pardir discussion elsewhere)
                                self.info_message("Refreshed directory listing")?;
                            }

                            Action::Help => self.help_view_loop()?,

                            Action::Exit => break Ok(()),
                            Action::ExitWithoutCd => {
                                // exit with error (ctl+c by default), to avoid cd'ing
                                let msg = format!("{}: Exited without changing folder",
                                                  env!("CARGO_PKG_NAME"));
                                break Err(TereError::ExitWithoutCd(msg));
                            }

                            Action::None => (),

                        }
                    } else {
                        // The key is not part of any mapping, advance the search if it's a char
                        if let KeyEvent { code: KeyCode::Char(c), .. } = k {
                            self.on_search_char(c)?;
                        } // else { self.info_message(&format!("{:?}", k))? } // for debugging
                    }
                }

                Event::Resize(_, _) => {
                    self.update_main_window_dimensions()?;
                    self.redraw_all_windows()?;
                }

                Event::Mouse(event) => match event.kind {
                    MouseEventKind::Down(MouseButton::Left)
                        | MouseEventKind::Drag(MouseButton::Left)
                        | MouseEventKind::Up(MouseButton::Left)
                        => self.handle_mouse_event(event)?,
                    MouseEventKind::Up(MouseButton::Right) => { self.change_dir("..")?; },

                    //TODO: add configuration to jump multiple items on scroll
                    MouseEventKind::ScrollUp   => self.on_cursor_up_down(true)?,
                    MouseEventKind::ScrollDown => self.on_cursor_up_down(false)?,

                    //e => self.info_message(&format!("{:?}", e))?, // for debugging
                    _ => (),
                },
            }
        };

        if self.app_state.settings().mouse_enabled {
            execute!(self.window, DisableMouseCapture)?;
        }

        self.app_state.on_exit().map_err(TereError::from)
            .and(loop_result)
    }

    fn help_view_loop(&mut self) -> CTResult<()> {
        self.info_message("Use ↓/↑ or j/k to scroll. Press Esc, 'q', '?' or Ctrl+c to exit help.")?;

        // We don't need the help view scroll state anywhere else, so not worth it to put in
        // app_state, just keep it here.
        let mut help_view_scroll: usize = 0;

        // drawing the help takes care of clearing the window
        self.draw_help_view(help_view_scroll)?;

        loop {
            match read_event()? {
                Event::Key(k) => match k.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                        self.info_message("")?;
                        return self.redraw_all_windows();
                    }

                    KeyCode::Char('c') if k.modifiers == KeyModifiers::CONTROL => {
                        self.info_message("")?;
                        return self.redraw_all_windows();
                    }

                    KeyCode::Down | KeyCode::Char('j') => {
                        help_view_scroll += 1;
                        self.draw_help_view(help_view_scroll)?;
                    }

                    KeyCode::Up | KeyCode::Char('k') => {
                        help_view_scroll = help_view_scroll.saturating_sub(1);
                        self.draw_help_view(help_view_scroll)?;
                    }

                    _ => {}
                },

                Event::Resize(_, _) => {
                    self.update_main_window_dimensions()?;
                    // Redraw all windows except for main window
                    self.redraw_header()?;
                    self.redraw_info_window()?;
                    self.redraw_footer()?;
                    self.draw_help_view(help_view_scroll)?;
                }

                _ => {}
            }
        }
    }

    fn draw_help_view(&mut self, scroll: usize) -> CTResult<()> {
        queue!(
            self.window,
            cursor::MoveTo(0, u16::try_from(HEADER_SIZE).unwrap_or(u16::MAX)),
            style::SetAttribute(Attribute::Reset),
            style::ResetColor,
        )?;

        let (w, h) = main_window_size()?;
        let help_text = get_formatted_help_text(w, &self.app_state.settings().keymap);
        for (i, line) in help_text
            .iter()
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
                style::Print(if i == 0 { "" } else { "\n" }),
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
