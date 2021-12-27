//! Background processes management.

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::process::Child;
use std::sync::Mutex;

/// adds a process to the pool of processes we manage.
pub fn add(proc: Child) {
    MANAGER.lock().unwrap().add(proc);
}

/// adds some processes to the pool of processes we manage.
pub fn addq(procs: VecDeque<Child>) {
    MANAGER.lock().unwrap().addq(procs);
}

/// collects terminated processes.
pub fn collect() {
    MANAGER.lock().unwrap().collect();
}

/// Manages background processes.
struct Manager {
    pub procs: VecDeque<Child>,
}

/// The global process manager.
static MANAGER: Lazy<Mutex<Manager>> = Lazy::new(|| Mutex::new(Manager::new()));

impl Manager {
    /// creates a new process manager.
    fn new() -> Manager {
        Manager {
            procs: VecDeque::<_>::new(),
        }
    }

    /// adds a process to the pool of processes we manage.
    fn add(self: &mut Self, proc: Child) {
        self.procs.push_back(proc);
    }

    /// adds some processes to the pool of processes we manage.
    pub fn addq(self: &mut Self, mut procs: VecDeque<Child>) {
        while procs.len() > 0 {
            self.add(procs.pop_front().unwrap()); // cannot fail
        }
    }

    /// collects terminated processes.
    fn collect(self: &mut Self) {
        let mut running = VecDeque::<Child>::new();
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
