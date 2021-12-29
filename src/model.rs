//! Common data model.

/// Error emitted by the shell.
#[derive(Debug)]
pub struct Error {
    reason: String,
}

/// Result of an operation.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Creates a new instance of error.
    pub fn new(reason: &str) -> Error {
        Error {
            reason: String::from(reason),
        }
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.reason)
    }
}
