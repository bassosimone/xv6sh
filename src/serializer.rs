//! Serializes parse tree to shell syntax

use crate::model::{Error, Result};
use crate::parser;

/// Serializes the parse tree to string.
pub fn serialize(cc: parser::CompleteCommand) -> Result<String> {
    let mut serializer = Serializer::new();
    serializer.complete_command(cc)?;
    return Ok(serializer.out);
}

/// Implements serialization.
struct Serializer {
    out: String,
}

impl Serializer {
    /// creates a new serializer instance.
    fn new() -> Serializer {
        Serializer { out: String::new() }
    }

    /// visits each pipeline inside the complete command.
    fn complete_command(self: &mut Self, cc: parser::CompleteCommand) -> Result<()> {
        let mut pipelines = cc.pipelines;
        loop {
            match pipelines.pop_front() {
                None => break,
                Some(p) => {
                    self.pipeline(p)?;
                    if pipelines.len() > 0 {
                        self.out.push(';');
                    }
                }
            }
        }
        Ok(())
    }

    /// visits each command inside the pipeline.
    fn pipeline(self: &mut Self, pipeline: parser::Pipeline) -> Result<()> {
        let mut commands = pipeline.commands;
        if commands.len() <= 0 {
            return Err(Error::new("empty pipeline"));
        }
        loop {
            match commands.pop_front() {
                None => break,
                Some(cmd) => {
                    self.command(cmd)?;
                    if commands.len() > 0 {
                        self.out.push('|');
                    }
                }
            }
        }
        Ok(())
    }

    /// visits a specific command
    fn command(self: &mut Self, command: parser::Command) -> Result<()> {
        match command {
            parser::Command::SimpleCommand(cmd) => self.simple_command(cmd),
            parser::Command::Subshell(ss) => self.subshell(ss),
        }
    }

    /// visits a simple command
    fn simple_command(self: &mut Self, sc: parser::SimpleCommand) -> Result<()> {
        let mut arguments = sc.arguments;
        loop {
            match arguments.pop_front() {
                None => break,
                Some(argument) => {
                    self.out.push_str(&argument);
                    if arguments.len() > 0 {
                        self.out.push(' ');
                    }
                }
            }
        }
        self.redirs(sc.redirs)?;
        Ok(())
    }

    /// visits a subshell
    fn subshell(self: &mut Self, ss: parser::Subshell) -> Result<()> {
        self.out.push('(');
        self.complete_command(ss.complete_command)?;
        self.out.push(')');
        self.redirs(ss.redirs)
    }

    // TODO(bassosimone): the serializer should probably fail to
    // serialize if we have multiple i/o redirections. Because of
    // how the shell works, we cannot handle more than a single
    // output and a single output redirection.
    //
    // If we don't do that, the error indicating we have multiple
    // redirections is instead emitted by a subshell.

    /// visit redirs
    fn redirs(self: &mut Self, redirs: parser::RedirectList) -> Result<()> {
        if redirs.input.len() > 0 {
            self.out.push('<');
            self.out.push_str(&redirs.input[0].filename);
        }
        if redirs.output.len() > 0 {
            if redirs.output[0].overwrite {
                self.out.push('>')
            } else {
                self.out.push_str(">>")
            }
            self.out.push_str(&redirs.output[0].filename);
        }
        Ok(())
    }
}
