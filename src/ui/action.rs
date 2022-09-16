use std::cmp::{PartialEq, Eq};
use std::hash::Hash;
use strum_macros::{
    Display as StrumDisplay,
    EnumString,
    EnumIter,
};

/// The possible actions that the user can do


#[derive(Debug, PartialEq, Eq, Clone, StrumDisplay, EnumString, EnumIter)]
pub enum Action {
    ChangeDir,
    ChangeDirParent,
    ChangeDirHome,
    ChangeDirRoot,
    ChangeDirAndExit,

    CursorUp,
    CursorDown,
    CursorUpScreen,
    CursorDownScreen,
    CursorTop,
    CursorBottom,

    EraseSearchChar,
    ClearSearch,

    ChangeFilterSearchMode,
    ChangeCaseSensitiveMode,
    ChangeGapSearchMode,
    ChangeAttributeSortMode,

    RefreshListing,

    Help,

    Exit,
    ExitWithoutCd,

    None,
}

impl Action {
    pub fn description(&self) -> &'static str {
        match self {
            Self::ChangeDir => "Enter directory under the cursor",
            Self::ChangeDirParent => "Go to the parent directory",
            Self::ChangeDirHome => "Go to the home directory",
            Self::ChangeDirRoot => "Go to the root directory",
            Self::ChangeDirAndExit => "Enter the directory under the cursor and exit",

            Self::CursorUp => "Move the cursor up by one step",
            Self::CursorDown => "Move the cursor down by one step",
            Self::CursorUpScreen => "Move the cursor up by one screenful",
            Self::CursorDownScreen => "Move the cursor down by one screenful",
            Self::CursorTop => "Move the cursor to the first item in the listing",
            Self::CursorBottom => "Move the cursor to the last item in the listing",

            Self::EraseSearchChar => "Erase one character from the search",
            Self::ClearSearch => "Clear the search",

            Self::ChangeFilterSearchMode => "Toggle the filter-search mode",
            Self::ChangeCaseSensitiveMode => "Change the case-sensitive mode",
            Self::ChangeGapSearchMode => "Change the gap-search mode",
            Self::ChangeAttributeSortMode => "Change the sorting mode",

            Self::RefreshListing => "Refresh the directory listing",

            Self::Help => "Show the help screen",

            Self::Exit => "Exit the program",
            Self::ExitWithoutCd => "Exit the program without changing the working directory",

            Self::None => "Disable this mapping",
        }
    }
}

/// An extra quantifier on an action, like 'this only applies when searching'
#[derive(Hash, PartialEq, Eq, Clone, Debug, StrumDisplay, EnumString, EnumIter)]
pub enum ActionContext {
    /// Signifies that this shortcut should apply if no other condition applies
    None,

    /// This shortcut only applies when searching
    Searching,

    /// This shortcut only applies when not searching
    NotSearching,
}

impl ActionContext {
    /// Description of this context to use in the output of --help
    pub fn description(&self) -> &'static str {
        match self {
            Self::None => "This mapping applies if no other context applies. This is the behavior if no context is specified: the mapping 'key-combination:action' is equivalent to 'key-combination:None:action'.",
            Self::Searching => "This mapping only applies while searching (at least one search character has been given).",
            Self::NotSearching => "This mapping only applies while not searching.",
        }
    }

    /// Half-way between the serialization provided by strum::Display and self.description(): a
    /// short human-readable string.
    pub fn short_description(&self) -> &'static str {
        match self {
            Self::None => "no context",
            Self::Searching => "when searching",
            Self::NotSearching => "when not searching",
        }
    }
}
