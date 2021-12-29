//! Processes management code.

use crate::model::{Error, ProcessExecutor, ProcessManager, Result};
use std::collections::VecDeque;
use std::process::{Child, Command};

/// A Manager for background processes.
pub struct Manager {
    procs: VecDeque<Child>,
}

impl Manager {
    /// creates a new process manager.
    pub fn new() -> Manager {
        Manager {
            procs: VecDeque::<_>::new(),
        }
    }

    /// adds a process to the pool of processes we manage.
    fn add(self: &mut Self, proc: Child) {
        self.procs.push_back(proc);
    }

    /// collects terminated processes.
    pub fn collect(self: &mut Self) {
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

impl<'a> ProcessManager<'a> for Manager {
    /// Creates a new Executor for this manager.
    fn new_executor(self: &'a mut Self) -> Box<dyn ProcessExecutor + 'a> {
        Box::new(Executor::new(self))
    }
}

/// Executes one or more processes.
pub struct Executor<'a> {
    children: VecDeque<Child>,
    manager: &'a mut Manager,
}

impl<'a> Executor<'a> {
    /// Creates a new Executor.
    fn new(manager: &'a mut Manager) -> Executor<'a> {
        Executor {
            children: VecDeque::<_>::new(),
            manager: manager,
        }
    }
}

impl<'a> ProcessExecutor for Executor<'a> {
    fn spawn(self: &mut Self, mut cmd: Command) -> Result<()> {
        match cmd.spawn() {
            Err(err) => return Err(Error::new(&err.to_string())),
            Ok(child) => {
                self.children.push_back(child);
                Ok(())
            }
        }
    }

    fn kill_children(self: &mut Self) {
        for c in self.children.iter_mut() {
            let _ = c.kill(); // ignore return value
        }
        self.wait_for_children();
    }

    fn wait_for_children(self: &mut Self) {
        while self.children.len() > 0 {
            // note: proceed backwards
            let mut c = self.children.pop_back().unwrap(); // cannot fail
            let _ = c.wait(); // ignore return value
        }
    }
}

impl<'a> Drop for Executor<'a> {
    /// ensures we kill the children at a later time.
    fn drop(&mut self) {
        while self.children.len() > 0 {
            self.manager.add(self.children.pop_front().unwrap());
        }
    }
}
