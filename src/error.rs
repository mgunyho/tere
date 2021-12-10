/// Custom error type
pub enum TereError {
    IoError(std::io::Error),
    ClapError(clap::Error),
    SerdeJsonError(serde_json::error::Error),
}


impl From<std::io::Error> for TereError {
    fn from(e: std::io::Error) -> Self { Self::IoError(e) }
}

impl From<clap::Error> for TereError {
    fn from(e: clap::Error) -> Self { Self::ClapError(e) }
}

impl From<serde_json::error::Error> for TereError {
    fn from(e: serde_json::error::Error) -> Self { Self::SerdeJsonError(e) }
}
