//! Interprets the executable syntax tree generated
//! by the translator module (translator.rs).

use crate::model::{Error, ProcessSpawner, Result};
use crate::parser::{InputRedir, OutputRedir};
use crate::process::{Group, PeriodicReaper, Spawner};
use crate::translator::{
    CompoundSerialCommand, FilterCommand, ListOfCommands, PipelinedCommands, SingleCommand,
    SinkCommand, SourceCommand,
};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::collections::VecDeque;
use std::convert::Into;
use std::fs::{File, OpenOptions};
use std::process::{Command, Stdio};

/// Interprets the given ListOfCommands
pub struct Interpreter {
    spawner: Box<dyn ProcessSpawner>,
    verbose: bool,
}

impl Interpreter {
    /// Creates a new interpreter.
    pub fn new(verbose: bool) -> Interpreter {
        Self::new_with_spawner(verbose, Spawner::new())
    }

    /// Creates a new interpreter with the given spawner.
    pub fn new_with_spawner(verbose: bool, spawner: Box<dyn ProcessSpawner>) -> Interpreter {
        Interpreter {
            spawner: spawner,
            verbose: verbose,
        }
    }

    /// Runs the interpreter
    pub fn run(self: &Self, mut loc: ListOfCommands, reaper: &mut PeriodicReaper) -> Result<()> {
        loop {
            match loc.pipelines.pop_front() {
                None => return Ok(()),
                Some(p) => {
                    self.compound_serial_command(p, reaper)?;
                    continue;
                }
            }
        }
    }

    /// Executes a CompoundSerialCommand
    fn compound_serial_command(
        self: &Self,
        csc: CompoundSerialCommand,
        reaper: &mut PeriodicReaper,
    ) -> Result<()> {
        match csc {
            CompoundSerialCommand::SingleCommand(sc) => self.single_command(sc, reaper),
            CompoundSerialCommand::PipelinedCommands(pc) => self.pipelined_commands(pc, reaper),
        }
    }

    /// Executes a SingleCommand
    fn single_command(
        self: &Self,
        mut sc: SingleCommand,
        reaper: &mut PeriodicReaper,
    ) -> Result<()> {
        // Implementation note: we only check for builtin commands
        // when we're not in pipeline context - is this correct?
        if sc.arguments.len() < 1 {
            // we arrive here when we hit [Enter] at the prompt
            //eprintln!("bonsoir, Elliot!");
            return Ok(());
        }
        let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
        match argv0.as_str() {
            "cd" => {
                return Self::builtin_cd(sc.arguments);
            }
            _ => (),
        }
        let rin = Self::maybe_redirect_input(&sc.input)?;
        let rout = Self::maybe_redirect_output(&sc.output)?;
        let mut group = Group::new(reaper);
        self.exec(&mut group, argv0, sc.arguments, rin, rout)?;
        if sc.sync {
            group.wait();
        }
        Ok(())
    }

    /// Implements the builtin `cd` command
    fn builtin_cd(args: VecDeque<String>) -> Result<()> {
        // TODO(bassosimone): `cd` without arguments should bring
        // the user to the home directory...
        if args.len() != 1 {
            return Err(Error::new("usage: cd <directory>"));
        }
        match std::env::set_current_dir(&args[0]) {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok(_) => Ok(()),
        }
    }

    /// Executes a pipeline of commands with at least a source and a sink
    fn pipelined_commands(
        self: &Self,
        pc: PipelinedCommands,
        reaper: &mut PeriodicReaper,
    ) -> Result<()> {
        let mut rxall = VecDeque::<PipeReader>::new();
        let mut group = Group::new(reaper);
        let source = pc.source;
        let rx = self.source_command(&mut group, source)?;
        rxall.push_back(rx);
        for filter in pc.filters {
            match self.filter_command(&mut group, filter, rxall.pop_back().unwrap()) {
                Err(err) => {
                    group.kill_and_wait();
                    return Err(Error::new(&err.to_string()));
                }
                Ok(rx) => rxall.push_back(rx),
            }
        }
        match self.sink_command(&mut group, pc.sink, rxall.pop_back().unwrap()) {
            Err(err) => {
                group.kill_and_wait();
                return Err(Error::new(&err.to_string()));
            }
            Ok(_) => (),
        }
        if pc.sync {
            group.wait();
        }
        Ok(())
    }

    /// Executes the source command of the pipeline
    fn source_command(self: &Self, group: &mut Group, mut sc: SourceCommand) -> Result<PipeReader> {
        if sc.arguments.len() < 1 {
            return Err(Error::new("pipeline with empty source command"));
        }
        let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
        let rin = Self::maybe_redirect_input(&sc.input)?;
        let (crx, cwx) = Self::wrap_os_pipe()?;
        match self.exec(group, argv0, sc.arguments, rin, Some(cwx)) {
            Err(err) => Err(err),
            Ok(_) => Ok(crx),
        }
    }

    /// Executes a filter command of a pipeline
    fn filter_command(
        self: &Self,
        group: &mut Group,
        mut fc: FilterCommand,
        rx: PipeReader,
    ) -> Result<PipeReader> {
        if fc.arguments.len() < 1 {
            return Err(Error::new("pipeline with empty filter command"));
        }
        let argv0 = fc.arguments.pop_front().unwrap(); // cannot fail
        let (crx, cwx) = Self::wrap_os_pipe()?;
        match self.exec(group, argv0, fc.arguments, Some(rx), Some(cwx)) {
            Err(err) => Err(err),
            Ok(_) => Ok(crx),
        }
    }

    /// Executes the sink command of a pipeline
    fn sink_command(
        self: &Self,
        group: &mut Group,
        mut sc: SinkCommand,
        rx: PipeReader,
    ) -> Result<()> {
        if sc.arguments.len() < 1 {
            return Err(Error::new("pipeline with empty sink command"));
        }
        let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
        let rou = Self::maybe_redirect_output(&sc.output)?;
        self.exec(group, argv0, sc.arguments, Some(rx), rou)
    }

    /// Creates the input redirection if needed.
    fn maybe_redirect_input(input: &Option<InputRedir>) -> Result<Option<File>> {
        match input {
            None => Ok(None),
            Some(input) => match File::open(&input.filename) {
                Err(err) => Err(Error::new(&err.to_string())),
                Ok(filep) => Ok(Some(filep)),
            },
        }
    }

    /// Creates the output redirection if needed.
    fn maybe_redirect_output(output: &Option<OutputRedir>) -> Result<Option<File>> {
        match output {
            None => Ok(None),
            Some(output) => match OpenOptions::new()
                .write(true)
                .create(true)
                .append(!output.overwrite)
                .open(&output.filename)
            {
                Err(err) => return Err(Error::new(&err.to_string())),
                Ok(filep) => Ok(Some(filep)),
            },
        }
    }

    /// Common code for executing a child process.
    fn exec<T1: Into<Stdio>, T2: Into<Stdio>>(
        self: &Self,
        group: &mut Group,
        argv0: String,
        mut args: VecDeque<String>,
        stdin: Option<T1>,
        stdout: Option<T2>,
    ) -> Result<()> {
        self.maybe_debug(&argv0, &args);
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
        let proc = self.spawner.spawn(cmd)?;
        group.add(proc); // ensure we track the child
        Ok(())
    }

    /// Possibly log to stderr the commands we're about to execute.
    fn maybe_debug(self: &Self, argv0: &str, args: &VecDeque<String>) {
        if self.verbose {
            let mut farg = String::new();
            for arg in args.iter() {
                farg.push_str(arg);
                farg.push(' ');
            }
            eprintln!("+ {} {}", argv0, farg);
        }
    }

    /// Wrapper to adapt os_pipe::pipe to our kind of Result
    fn wrap_os_pipe() -> Result<(PipeReader, PipeWriter)> {
        match pipe() {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok((rx, wx)) => Ok((rx, wx)),
        }
    }
}
