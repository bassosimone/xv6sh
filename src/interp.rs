//! Interprets the executable syntax tree.

use crate::model::{Error, Result};
use crate::translator::{
    CompoundSerialCommand, FilterCommand, ListOfCommands, PipelinedCommands, SingleCommand,
    SinkCommand, SourceCommand,
};
use os_pipe::{pipe, PipeReader, PipeWriter};
use std::collections::VecDeque;
use std::fs::{File, OpenOptions};
use std::process::{Child, Command};

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

fn compound_serial_command(csc: CompoundSerialCommand) -> Result<()> {
    match csc {
        CompoundSerialCommand::SingleCommand(sc) => single_command(sc),
        CompoundSerialCommand::PipelinedCommands(pc) => pipelined_commands(pc),
    }
}

fn single_command(mut sc: SingleCommand) -> Result<()> {
    if sc.arguments.len() < 1 {
        return Ok(());
    }
    let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
    let mut cmd = Command::new(argv0);
    while sc.arguments.len() > 0 {
        let arg = sc.arguments.pop_front().unwrap(); // cannot fail
        cmd.arg(arg);
    }
    match sc.input {
        None => (),
        Some(input) => match File::open(input.filename) {
            Err(err) => return Err(Error::new(&err.to_string())),
            Ok(filep) => {
                cmd.stdin(filep);
            }
        },
    }
    match sc.output {
        None => (),
        Some(output) => match OpenOptions::new()
            .write(true)
            .create(true)
            .append(!output.overwrite)
            .open(output.filename)
        {
            Err(err) => return Err(Error::new(&err.to_string())),
            Ok(filep) => {
                cmd.stdout(filep);
            }
        },
    }
    match cmd.spawn() {
        Err(err) => return Err(Error::new(&err.to_string())),
        Ok(mut child) => {
            let _ = child.wait(); // we don't care about the return value
            Ok(())
        }
    }
}

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
    wait_for_children(children);
    Ok(())
}

fn kill_children(mut children: VecDeque<Child>) {
    for c in children.iter_mut() {
        let _ = c.kill(); // ignore return value
    }
    wait_for_children(children);
}

fn wait_for_children(mut children: VecDeque<Child>) {
    while children.len() > 0 {
        let mut c = children.pop_back().unwrap(); // cannot fail
        let _ = c.wait(); // ignore return value
    }
}

fn source_command(mut sc: SourceCommand) -> Result<(Child, PipeReader)> {
    if sc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty source command"));
    }
    let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
    let mut cmd = Command::new(argv0);
    while sc.arguments.len() > 0 {
        let arg = sc.arguments.pop_front().unwrap(); // cannot fail
        cmd.arg(arg);
    }
    match sc.input {
        None => (),
        Some(input) => match File::open(input.filename) {
            Err(err) => return Err(Error::new(&err.to_string())),
            Ok(filep) => {
                cmd.stdin(filep);
            }
        },
    }
    let (rx, wx) = sys_pipe()?;
    cmd.stdout(wx);
    match cmd.spawn() {
        Err(err) => return Err(Error::new(&err.to_string())),
        Ok(child) => Ok((child, rx)),
    }
}

fn filter_command(mut fc: FilterCommand, rx: PipeReader) -> Result<(Child, PipeReader)> {
    if fc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty filter command"));
    }
    let argv0 = fc.arguments.pop_front().unwrap(); // cannot fail
    let mut cmd = Command::new(argv0);
    while fc.arguments.len() > 0 {
        let arg = fc.arguments.pop_front().unwrap(); // cannot fail
        cmd.arg(arg);
    }
    cmd.stdin(rx);
    let (rx, wx) = sys_pipe()?;
    cmd.stdout(wx);
    match cmd.spawn() {
        Err(err) => return Err(Error::new(&err.to_string())),
        Ok(child) => Ok((child, rx)),
    }
}

fn sink_command(mut sc: SinkCommand, rx: PipeReader) -> Result<Child> {
    if sc.arguments.len() < 1 {
        return Err(Error::new("pipeline with empty sink command"));
    }
    let argv0 = sc.arguments.pop_front().unwrap(); // cannot fail
    let mut cmd = Command::new(argv0);
    while sc.arguments.len() > 0 {
        let arg = sc.arguments.pop_front().unwrap(); // cannot fail
        cmd.arg(arg);
    }
    cmd.stdin(rx);
    match sc.output {
        None => (),
        Some(output) => match OpenOptions::new()
            .write(true)
            .create(true)
            .append(!output.overwrite)
            .open(output.filename)
        {
            Err(err) => return Err(Error::new(&err.to_string())),
            Ok(filep) => {
                cmd.stdout(filep);
            }
        },
    }
    match cmd.spawn() {
        Err(err) => return Err(Error::new(&err.to_string())),
        Ok(child) => Ok(child),
    }
}

fn sys_pipe() -> Result<(PipeReader, PipeWriter)> {
    match pipe() {
        Err(err) => Err(Error::new(&err.to_string())),
        Ok((rx, wx)) => Ok((rx, wx)),
    }
}
