//! Background processes management.

use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::process::Child;
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

/// Something that we could periodically check whether it has terminated.
trait TryWaitable {
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
