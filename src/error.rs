/// Custom error type
#[derive(Debug)]
pub enum TereError {
    Io(std::io::Error),
    Clap(clap::Error),
    SerdeJson(serde_json::error::Error),
}


impl From<std::io::Error> for TereError {
    fn from(e: std::io::Error) -> Self { Self::Io(e) }
}

impl From<clap::Error> for TereError {
    fn from(e: clap::Error) -> Self { Self::Clap(e) }
}

impl From<serde_json::error::Error> for TereError {
    fn from(e: serde_json::error::Error) -> Self { Self::SerdeJson(e) }
}
