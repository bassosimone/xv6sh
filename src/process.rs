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

/// collects terminated processes.
pub fn collect() {
    MANAGER.lock().unwrap().collect();
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

/// Executes one or more processes.
pub struct Executor {
    children: VecDeque<Child>,
}

impl Executor {
    /// Creates a new Executor.
    pub fn new() -> Executor {
        Executor {
            children: VecDeque::<_>::new(),
        }
    }

    /// Common code for spawning a child process.
    pub fn spawn<T1: Into<Stdio>, T2: Into<Stdio>>(
        self: &mut Self,
        argv0: String,
        mut args: VecDeque<String>,
        stdin: Option<T1>,
        stdout: Option<T2>,
    ) -> Result<()> {
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
            Ok(child) => {
                self.children.push_back(child);
                Ok(())
            }
        }
    }

    /// Kills all the children inside a pipeline
    pub fn kill_children(self: &mut Self) {
        for c in self.children.iter_mut() {
            let _ = c.kill(); // ignore return value
        }
        self.wait_for_children();
    }

    /// Waits for pipeline children to terminate
    pub fn wait_for_children(self: &mut Self) {
        while self.children.len() > 0 {
            // note: proceed backwards
            let mut c = self.children.pop_back().unwrap(); // cannot fail
            let _ = c.wait(); // ignore return value
        }
    }
}

impl Drop for Executor {
    /// ensures we kill the children at a later time.
    fn drop(&mut self) {
        // TODO(bassosimone): this is currently very implicit
        while self.children.len() > 0 {
            add(self.children.pop_front().unwrap());
        }
    }
}
