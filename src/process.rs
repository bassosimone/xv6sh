//! Processes management code.

use crate::model::{Error, Result};
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

/// adds a process to the pool of processes we manage.
pub fn add(proc: Child) {
    MANAGER.lock().unwrap().add(ChildWrapper { child: proc });
}

/// converts a deque of Child to a deque of ChildWrapper.
fn to_child_wrapper(mut input: VecDeque<Child>) -> VecDeque<ChildWrapper> {
    let mut output = VecDeque::<ChildWrapper>::new();
    while input.len() > 0 {
        output.push_back(ChildWrapper {
            child: input.pop_front().unwrap(),
        })
    }
    output
}

/// adds some processes to the pool of processes we manage.
pub fn addq(procs: VecDeque<Child>) {
    MANAGER.lock().unwrap().addq(to_child_wrapper(procs));
}

/// collects terminated processes.
pub fn collect() {
    MANAGER.lock().unwrap().collect();
}

/// Common code for spawning a child process.
pub fn spawn<T1: Into<Stdio>, T2: Into<Stdio>>(
    argv0: String,
    mut args: VecDeque<String>,
    stdin: Option<T1>,
    stdout: Option<T2>,
) -> Result<Child> {
    let mut cmd = Command::new(argv0);
    while args.len() > 0 {
        let arg = args.pop_front().unwrap(); // cannot fail
        cmd.arg(arg);
    }
    if let Some(filep) = stdin {
        cmd.stdin(filep);
    }
    if let Some(filep) = stdout {
        cmd.stdout(filep);
    }
    match cmd.spawn() {
        Err(err) => return Err(Error::new(&err.to_string())),
        Ok(child) => Ok(child),
    }
}

/// Something that we could periodically check whether it has terminated.
pub trait TryWaitable {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>>;
}

/// Wraps a Child process.
struct ChildWrapper {
    pub child: Child,
}

impl TryWaitable for ChildWrapper {
    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

/// A Manager for background processes.
struct Manager<T: TryWaitable> {
    pub procs: VecDeque<T>,
}

/// The global process manager.
static MANAGER: Lazy<Mutex<Manager<ChildWrapper>>> = Lazy::new(|| Mutex::new(Manager::new()));

impl<T: TryWaitable> Manager<T> {
    /// creates a new process manager.
    fn new() -> Manager<T> {
        Manager {
            procs: VecDeque::<_>::new(),
        }
    }

    /// adds a process to the pool of processes we manage.
    fn add(self: &mut Self, proc: T) {
        self.procs.push_back(proc);
    }

    /// adds some processes to the pool of processes we manage.
    pub fn addq(self: &mut Self, mut procs: VecDeque<T>) {
        while procs.len() > 0 {
            self.add(procs.pop_front().unwrap()); // cannot fail
        }
    }

    /// collects terminated processes.
    fn collect(self: &mut Self) {
        let mut running = VecDeque::<_>::new();
        while self.procs.len() > 0 {
            let mut cur = self.procs.pop_front().unwrap(); // cannot fail
            match cur.try_wait() {
                Err(_) => (),
                Ok(Some(_)) => (),
                Ok(None) => {
                    running.push_back(cur);
                }
            }
        }
        self.procs = running;
    }
}
