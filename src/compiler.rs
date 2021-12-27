//! Compiles shell commands to bytecode.

use crate::model::{Error, Result};
use crate::parser;
use crate::serializer;
use std::collections::VecDeque;

/// A command executed by the shell VM.
#[derive(Debug)]
pub struct OpCommand {
    pub arguments: VecDeque<String>,
    pub stdin: StdinPolicy,
    pub stdout: StdoutPolicy,
}

/// Policy for the standard input.
#[derive(Debug, PartialEq)]
pub enum StdinPolicy {
    Inherit,
    Pipe,
    ReadFrom(String),
}

/// Policy for the standard output.
#[derive(Debug, PartialEq)]
pub enum StdoutPolicy {
    Inherit,
    Pipe,
    OverwriteTo(String),
    AppendTo(String),
}

const REDIR_STDIN: u32 = 1 << 0;
const REDIR_STDOUT: u32 = 1 << 1;

/// Compiles the parse tree to bytecode.
pub fn compile(cc: parser::CompleteCommand) -> Result<VecDeque<OpCommand>> {
    let mut compiler = Compiler::new();
    compiler.complete_command(cc)?;
    return Ok(compiler.out);
}

/// The compiler implementation.
struct Compiler {
    out: VecDeque<OpCommand>,
}

impl Compiler {
    /// creates a new compiler
    fn new() -> Compiler {
        Compiler {
            out: VecDeque::<_>::new(),
        }
    }

    /// visits each pipeline inside the complete command.
    fn complete_command(self: &mut Self, cc: parser::CompleteCommand) -> Result<()> {
        let mut pipelines = cc.pipelines;
        loop {
            match pipelines.pop_front() {
                None => break,
                Some(p) => self.pipeline(p)?,
            }
        }
        Ok(())
    }

    /// visits each command inside the pipeline.
    fn pipeline(self: &mut Self, pipeline: parser::Pipeline) -> Result<()> {
        let mut commands = pipeline.commands;
        if commands.len() <= 0 {
            return Err(Error::new("sh: empty pipeline"));
        }
        let mut flags: u32 = 0;
        loop {
            match commands.pop_front() {
                None => break,
                Some(cmd) => {
                    if commands.len() > 0 {
                        flags |= REDIR_STDOUT;
                    } else {
                        flags &= !REDIR_STDOUT;
                    }
                    self.command(cmd, flags)?
                }
            }
            flags |= REDIR_STDIN;
        }
        Ok(())
    }

    /// visits a specific command
    fn command(self: &mut Self, command: parser::Command, flags: u32) -> Result<()> {
        match command {
            parser::Command::SimpleCommand(cmd) => self.simple_command(cmd, flags),
            parser::Command::Subshell(ss) => self.subshell(ss, flags),
        }
    }

    /// visits a simple command
    fn simple_command(self: &mut Self, sc: parser::SimpleCommand, flags: u32) -> Result<()> {
        let mut cmd = OpCommand {
            arguments: sc.arguments,
            stdin: StdinPolicy::Inherit,
            stdout: StdoutPolicy::Inherit,
        };
        if sc.redirs.input.len() > 0 && (flags & REDIR_STDIN) != 0 {
            return Err(Error::new("sh: input redirection used inside pipeline"));
        } else if sc.redirs.input.len() > 0 {
            cmd.stdin = StdinPolicy::ReadFrom(sc.redirs.input[0].filename.clone())
        } else if (flags & REDIR_STDIN) != 0 {
            cmd.stdin = StdinPolicy::Pipe;
        }
        if sc.redirs.output.len() > 0 && (flags & REDIR_STDOUT) != 0 {
            return Err(Error::new("sh: output redirection used inside pipeline"));
        } else if sc.redirs.output.len() > 0 {
            let ro = &sc.redirs.output[0];
            if ro.overwrite {
                cmd.stdout = StdoutPolicy::OverwriteTo(ro.filename.clone())
            } else {
                cmd.stdout = StdoutPolicy::AppendTo(ro.filename.clone())
            }
        } else if (flags & REDIR_STDOUT) != 0 {
            cmd.stdout = StdoutPolicy::Pipe;
        }
        self.out.push_back(cmd);
        Ok(())
    }

    /// visits a subshell
    fn subshell(self: &mut Self, ss: parser::Subshell, flags: u32) -> Result<()> {
        let mut scmd = parser::SimpleCommand {
            arguments: VecDeque::<_>::new(),
            redirs: parser::RedirectList::new(),
        };
        let exe = Self::get_current_exe()?;
        scmd.arguments.push_back(exe);
        scmd.arguments.push_back(String::from("-c"));
        let serialized = serializer::serialize(ss.complete_command)?;
        scmd.arguments.push_back(serialized);
        self.simple_command(scmd, flags)
    }

    /// Helper function to obtain the current exe.
    fn get_current_exe() -> Result<String> {
        match std::env::current_exe() {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok(pb) => match pb.to_str() {
                None => Err(Error::new("sh: unicode decode error")),
                Some(path) => Ok(String::from(path)),
            },
        }
    }
}
