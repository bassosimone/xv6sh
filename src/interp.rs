//! Interprets the executable syntax tree generated
//! by the translator module (translator.rs).

use crate::background;
use crate::model::{Error, Result};
use crate::parser::{InputRedir, OutputRedir};
use crate::translator::{
    CompoundSerialCommand, FilterCommand, ListOfCommands, PipelinedCommands, SingleCommand,
    SinkCommand, SourceCommand,
};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::collections::VecDeque;
use std::convert::Into;
use std::fs::{File, OpenOptions};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Interprets the given ListOfCommands
pub fn interpret(mut loc: ListOfCommands) -> Result<()> {
    loop {
        match loc.pipelines.pop_front() {
            None => return Ok(()),
            Some(p) => {
                compound_serial_command(p)?;
                continue;
            }
        }
    }
}

/// Executes a CompoundSerialCommand
fn compound_serial_command(csc: CompoundSerialCommand) -> Result<()> {
    match csc {
        CompoundSerialCommand::SingleCommand(sc) => single_command(sc),
        CompoundSerialCommand::PipelinedCommands(pc) => pipelined_commands(pc),
    }
}

/// Executes a SingleCommand
fn single_command(mut sc: SingleCommand) -> Result<()> {
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
            return builtin_cd(sc.arguments);
        }
        _ => (),
    }
    let rin = maybe_redirect_input(&sc.input)?;
    let rout = maybe_redirect_output(&sc.output)?;
    let mut chld = common_executor(argv0, sc.arguments, rin, rout)?;
    if sc.sync {
        let _ = chld.wait(); // we don't care about the return value
    } else {
        background::add(chld);
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
fn pipelined_commands(pc: PipelinedCommands) -> Result<()> {
    let mut children = VecDeque::<Child>::new();
    let mut rxall = VecDeque::<PipeReader>::new();
    let source = pc.source;
    let (child, rx) = source_command(source)?;
    children.push_back(child);
    rxall.push_back(rx);
    for filter in pc.filters {
        match filter_command(filter, rxall.pop_back().unwrap()) {
            Err(err) => {
                kill_children(children);
                return Err(Error::new(&err.to_string()));
            }
            Ok((child, rx)) => {
                children.push_back(child);
                rxall.push_back(rx);
            }
        }
    }
    match sink_command(pc.sink, rxall.pop_back().unwrap()) {
        Err(err) => {
            kill_children(children);
            return Err(Error::new(&err.to_string()));
        }
        Ok(child) => {
            children.push_back(child);
        }
    }
    if pc.sync {
        wait_for_children(children);
    } else {
        background::addq(children);
    }
    Ok(())
}

/// Kills all the children inside a pipeline
fn kill_children(mut children: VecDeque<Child>) {
    for c in children.iter_mut() {
        let _ = c.kill(); // ignore return value
    }
    wait_for_children(children);
}

/// Waits for pipeline children to terminate
fn wait_for_children(mut children: VecDeque<Child>) {
    while children.len() > 0 {
        // note: proceed backwards
        let mut c = children.pop_back().unwrap(); // cannot fail
        let _ = c.wait(); // ignore return value
    }
}

/// Executes the source command of the pipeline
fn source_command(mut sc: SourceCommand) -> Result<(Child, PipeReader)> {
    if sc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty source command"));
    }
    let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
    let rin = maybe_redirect_input(&sc.input)?;
    let (crx, cwx) = wrap_os_pipe()?;
    match common_executor(argv0, sc.arguments, rin, Some(cwx)) {
        Err(err) => Err(err),
        Ok(child) => Ok((child, crx)),
    }
}

/// Executes a filter command of a pipeline
fn filter_command(mut fc: FilterCommand, rx: PipeReader) -> Result<(Child, PipeReader)> {
    if fc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty filter command"));
    }
    let argv0 = fc.arguments.pop_front().unwrap(); // cannot fail
    let (crx, cwx) = wrap_os_pipe()?;
    match common_executor(argv0, fc.arguments, Some(rx), Some(cwx)) {
        Err(err) => Err(err),
        Ok(child) => Ok((child, crx)),
    }
}

/// Executes the sink command of a pipeline
fn sink_command(mut sc: SinkCommand, rx: PipeReader) -> Result<Child> {
    if sc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty sink command"));
    }
    let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
    let rou = maybe_redirect_output(&sc.output)?;
    common_executor(argv0, sc.arguments, Some(rx), rou)
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
fn common_executor<T1: Into<Stdio>, T2: Into<Stdio>>(
    argv0: String,
    mut args: VecDeque<String>,
    stdin: Option<T1>,
    stdout: Option<T2>,
) -> Result<Child> {
    maybe_debug(&argv0, &args);
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

static VERBOSE: AtomicUsize = AtomicUsize::new(0);

/// Returns whether the interpreter is verbose.
pub fn is_verbose() -> bool {
    return VERBOSE.load(Ordering::Acquire) > 0;
}

/// Configures the interpreter to be verbose.
pub fn set_verbose() {
    VERBOSE.store(1, std::sync::atomic::Ordering::SeqCst);
}

/// Possibly log to stderr the commands we're about to execute.
fn maybe_debug(argv0: &str, args: &VecDeque<String>) {
    if is_verbose() {
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
