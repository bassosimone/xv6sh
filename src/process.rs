//! Processes management code.

use crate::model::{Error, Process, ProcessSpawner, Result};
use std::collections::VecDeque;
use std::process::{Child, Command};

/// A child process implementing model::Process.
struct ChildProcess {
    child: Child,
}

impl Process for ChildProcess {
    fn kill(&mut self) -> std::io::Result<()> {
        self.child.kill()
    }

    fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }

    fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait()
    }
}

/// Spawner spawns processes.
pub struct Spawner {}

impl Spawner {
    /// Crates a new generic ProcessSpawner instance.
    pub fn new() -> Box<dyn ProcessSpawner> {
        Box::new(Spawner {})
    }
}

impl ProcessSpawner for Spawner {
    fn spawn(self: &Self, mut cmd: Command) -> Result<Box<dyn Process>> {
        match cmd.spawn() {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok(child) => Ok(Box::new(ChildProcess { child: child })),
        }
    }
}

/// PeriodicReaper periodically reaps zombie processes.
pub struct PeriodicReaper {
    c: VecDeque<Box<dyn Process>>,
}

impl PeriodicReaper {
    /// Creates a new PeriodicReaper.
    pub fn new() -> PeriodicReaper {
        PeriodicReaper {
            c: VecDeque::<_>::new(),
        }
    }

    /// Adds a process to the pool of background processes we manage.
    fn add(self: &mut Self, proc: Box<dyn Process>) {
        self.c.push_back(proc);
    }

    /// Reaps all the zombies processes.
    pub fn reap(self: &mut Self) {
        let mut running = VecDeque::<_>::new();
        while self.c.len() > 0 {
            let mut cur = self.c.pop_front().unwrap(); // cannot fail
            match cur.try_wait() {
                Err(_) => (),
                Ok(Some(_)) => (),
                Ok(None) => {
                    running.push_back(cur);
                }
            }
        }
        self.c = running;
    }
}

/// Group is a group of processes. It owns all the processes you
/// explicitly add to it using the add method. If you do not call
/// kill_and_wait or wait, the ownership of the processes in the
/// group is transferred to the PeriodicReaper when the Group
/// is dropped because it has gone out of the scope.
pub struct Group<'a> {
    c: VecDeque<Box<dyn Process>>,
    pr: &'a mut PeriodicReaper,
}

impl<'a> Group<'a> {
    /// Creates a new empty group of processes.
    pub fn new(pr: &'a mut PeriodicReaper) -> Group<'a> {
        Group {
            c: VecDeque::<_>::new(),
            pr: pr,
        }
    }

    /// Adds a process to the group.
    pub fn add(self: &mut Self, proc: Box<dyn Process>) {
        self.c.push_back(proc);
    }

    /// Kills each process in the group and then waits for each of them.
    pub fn kill_and_wait(self: &mut Self) {
        for p in self.c.iter_mut() {
            let _ = p.kill(); // ignore return value
        }
        self.wait();
    }

    /// Waits for each process in the group to terminate.
    pub fn wait(self: &mut Self) {
        while self.c.len() > 0 {
            // note: proceed backwards
            let mut p = self.c.pop_back().unwrap(); // cannot fail
            let _ = p.wait(); // ignore return value
        }
    }
}

impl<'a> Drop for Group<'a> {
    /// Transfers processes ownership to the PeriodicReaper.
    fn drop(&mut self) {
        while self.c.len() > 0 {
            self.pr.add(self.c.pop_front().unwrap());
        }
    }
}
