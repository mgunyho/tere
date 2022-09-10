use std::cmp::{PartialEq, Eq};
use std::hash::Hash;

/// The possible actions that the user can do


#[derive(Debug, PartialEq, Clone, strum_macros::Display, strum_macros::EnumString, strum_macros::EnumIter)]
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

    ChangeCaseSensitiveMode,
    ChangeGapSearchMode,

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

            Self::ChangeCaseSensitiveMode => "Change the case-sensitive mode",
            Self::ChangeGapSearchMode => "Change the gap-search mode",

            Self::RefreshListing => "Refresh the directory listing",

            Self::Help => "Show the help screen",

            Self::Exit => "Exit the program",
            Self::ExitWithoutCd => "Exit the program without changing the working directory",

            Self::None => "Disable this mapping",
        }
    }
}

/// A list of all of the possible actions, so that they can be programmatically included in the
/// help text etc.
pub const ALL_ACTIONS: &[Action] = &[
    Action::ChangeDir,
    Action::ChangeDirParent,
    Action::ChangeDirHome,
    Action::ChangeDirRoot,
    Action::ChangeDirAndExit,
    Action::CursorUp,
    Action::CursorDown,
    Action::CursorUpScreen,
    Action::CursorDownScreen,
    Action::CursorTop,
    Action::CursorBottom,
    Action::EraseSearchChar,
    Action::ClearSearch,
    Action::ChangeCaseSensitiveMode,
    Action::ChangeGapSearchMode,
    Action::RefreshListing,
    Action::Help,
    Action::Exit,
    Action::ExitWithoutCd,
    Action::None,
];

/// An extra quantifier on an action, like 'this only applies when searching'
#[derive(Hash, PartialEq, Eq, Clone, Debug, strum_macros::Display, strum_macros::EnumString)]
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

/// A list of all possible action contexts, so that they can be programmatically included in the
/// help text etc.
pub const ALL_ACTION_CONTEXTS: &[ActionContext] = &[
    ActionContext::None,
    ActionContext::Searching,
    ActionContext::NotSearching,
];
