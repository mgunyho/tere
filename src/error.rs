/// Custom error type
#[derive(Debug)]
pub enum TereError {
    // We get a dead code warning because the content of these errors are never read, but we want
    // to pass them to the user if there is an error so we want to keep them. So we'll ignore the
    // warning.
    #[allow(dead_code)]
    Io(std::io::Error),
    Clap(clap::Error),
    #[allow(dead_code)]
    SerdeJson(serde_json::error::Error),

    // This is raised when the user wants to exit tere without changing the folder. A bit of a hack
    // to define this here but this is simple enough.
    ExitWithoutCd(String),

    // The user cancelled the first-run prompt
    FirstRunPromptCancelled(String),
}

impl From<std::io::Error> for TereError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<clap::Error> for TereError {
    fn from(e: clap::Error) -> Self {
        Self::Clap(e)
    }
}

impl From<serde_json::error::Error> for TereError {
    fn from(e: serde_json::error::Error) -> Self {
        Self::SerdeJson(e)
    }
}
