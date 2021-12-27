//! Interprets the executable syntax tree.

use crate::model::{Error, Result};
use crate::translator::{CompoundSerialCommand, ListOfCommands, PipelinedCommands, SingleCommand};
use std::fs::{File, OpenOptions};
use std::process::Command;

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

fn pipelined_commands(_pc: PipelinedCommands) -> Result<()> {
    Err(Error::new("pipelines: not yet implemented"))
}
