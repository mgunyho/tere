use std::cmp::{PartialEq, Eq};
use std::hash::Hash;

/// The possible actions that the user can do


#[derive(Clone)]
pub enum Action {
    /// Change to the directory under the cursor
    ChangeDir,
    /// Change to the parent directory
    ChangeDirParent,
    ChangeDirHome,
    ChangeDirRoot,
    /// Change to the directory under the cursor and exit
    ChangeDirAndExit,

    CursorUp,
    CursorDown,
    /// Move cursor down by one page
    CursorUpPage,
    /// Move cursor up by one page
    CursorDownPage,
    /// Move cursor to first item
    CursorFirst,
    /// Move cursor to last item
    CursorLast,

    /// Erase a character from the search
    EraseSearchChar,
    /// Clear the current search
    ClearSearch,
    /// Clear the current search, or if it's empty, exit
    ClearSearchOrExit,

    /// Cycle the case sensitivity mode
    ChangeCaseSensitiveMode,
    /// Cycle the gap search mode
    ChangeGapSearchMode,

    RefreshListing,

    /// Show the help screen
    Help,

    Exit,
    /// Exit without changing the directory
    ExitWithoutCd,
}

/// An extra quantifier on an action, like 'this only applies when searching'
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ActionContext {
    /// Signifies that this shortcut should apply if no other condition applies
    Any,

    /// This shortcut only applies when searching
    Searching,

    /// This shortcut only applies when not searching
    NotSearching,
}
