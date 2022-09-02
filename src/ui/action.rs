use std::cmp::{PartialEq, Eq};
use std::hash::Hash;

/// The possible actions that the user can do


#[derive(Debug, PartialEq, Clone)]
pub enum Action {
    ChangeDir,
    ChangeDirParent,
    ChangeDirHome,
    ChangeDirRoot,
    ChangeDirAndExit,

    CursorUp,
    CursorDown,
    CursorUpPage,
    CursorDownPage,
    CursorFirst,
    CursorLast,

    EraseSearchChar,
    ClearSearch,

    ChangeCaseSensitiveMode,
    ChangeGapSearchMode,

    RefreshListing,

    Help,

    Exit,
    ExitWithoutCd,
}

impl Action {
    pub fn description(&self) -> &'static str {
        match self {
            Self::ChangeDir => "Change to the directory under the cursor",
            Self::ChangeDirParent => "Change to the parent directory",
            Self::ChangeDirHome => "Change to the home directory",
            Self::ChangeDirRoot => "Change to the root directory",
            Self::ChangeDirAndExit => "Change to the directory under the cursor and exit",

            Self::CursorUp => "Move the cursor up by one step",
            Self::CursorDown => "Move the cursor down by one step",
            Self::CursorUpPage => "Move the cursor up by one screenful",
            Self::CursorDownPage => "Move the cursor down by one screenful",
            Self::CursorFirst => "Move the cursor to the first item in the listing",
            Self::CursorLast => "Move the cursor to the last item in the listing",

            Self::EraseSearchChar => "Erase one character from the search",
            Self::ClearSearch => "Clear the search",

            Self::ChangeCaseSensitiveMode => "Change the case-sensitive mode",
            Self::ChangeGapSearchMode => "Change the gap-search mode",

            Self::RefreshListing => "Refresh the directory listing",

            Self::Help => "Show the help screen",

            Self::Exit => "Exit the program",
            Self::ExitWithoutCd => "Exit the program without changing the working directory",
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
    Action::CursorUpPage,
    Action::CursorDownPage,
    Action::CursorFirst,
    Action::CursorLast,
    Action::EraseSearchChar,
    Action::ClearSearch,
    Action::ChangeCaseSensitiveMode,
    Action::ChangeGapSearchMode,
    Action::RefreshListing,
    Action::Help,
    Action::Exit,
    Action::ExitWithoutCd,
];

/// An extra quantifier on an action, like 'this only applies when searching'
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ActionContext {
    /// Signifies that this shortcut should apply if no other condition applies
    None,

    /// This shortcut only applies when searching
    Searching,

    /// This shortcut only applies when not searching
    NotSearching,
}

impl ActionContext {
    pub fn description(&self) -> &'static str {
        match self {
            Self::None => "This mapping applies if no other context applies. This is the behavior if no context is specified in the mapping.",
            Self::Searching => "This mapping only applies while searching (at least one search character has been given).",
            Self::NotSearching => "This mapping only applies while not searching.",
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
