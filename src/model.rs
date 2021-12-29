//! Common data model.

use std::process::Command;

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

/// Process is a running child process.
pub trait Process {
    fn kill(&mut self) -> std::io::Result<()>;
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>>;
    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus>;
}

/// Anything that can spawn child processes.
pub trait ProcessSpawner {
    /// Spawns a new process from the given command.
    fn spawn(self: &Self, cmd: Command) -> Result<Box<dyn Process>>;
}
