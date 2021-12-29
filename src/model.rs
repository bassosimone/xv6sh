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

/// Manages processes executed by the shell.
pub trait ProcessManager<'a> {
    /// Creates a new Executor for this manager.
    fn new_executor(self: &'a mut Self) -> Box<dyn ProcessExecutor + 'a>;
}

/// Executes one or more processes.
pub trait ProcessExecutor {
    /// Common code for spawning a foreground child process. This process
    /// will be added to a list of processes managed by the current
    /// executor. You can interact with these processes using either
    /// kill_children or wait_for_children. If you do not call either
    /// of these functions, the children will become one of the background
    /// processes managed by the executor's ProcessManager.
    fn spawn(self: &mut Self, cmd: Command) -> Result<()>;

    /// Kills all the foreground children and waits for them to finish.
    fn kill_children(self: &mut Self);

    /// Waits for foreground children to terminate.
    fn wait_for_children(self: &mut Self);
}
