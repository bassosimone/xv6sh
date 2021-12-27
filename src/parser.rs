//! Parser implementation.
//!
//! The grammar has been simplified from the one published at
//! https://pubs.opengroup.org/onlinepubs/009604599/utilities/xcu_chap02.html#tag_02_10.

use crate::lexer;
use crate::model::{Error, Result};
use std::collections::VecDeque;

/// A complete command in the shell grammar:
///
///     CompleteCommand ::= CompleteCommand ";" Pipeline
///                       | CompleteCommand "&" Pipeline
///                       | Pipeline
#[derive(Debug)]
pub struct CompleteCommand {
    pub pipelines: VecDeque<Pipeline>,
}

/// A pipeline of commands in the shell grammar:
///
///     Pipeline ::= Pipeline "|" Command
///                | Command
#[derive(Debug)]
pub struct Pipeline {
    pub commands: VecDeque<Command>,
    pub sync: bool,
}

/// A command in the shell grammar:
///
///     Command ::= SimpleCommand
///               | Subshell
#[derive(Debug)]
pub enum Command {
    SimpleCommand(SimpleCommand),
    Subshell(Subshell),
}

/// A simple command in the shell grammar:
///
///     SimpleCommand ::= Arguments RedirectList
#[derive(Debug)]
pub struct SimpleCommand {
    pub arguments: VecDeque<String>,
    pub redirs: RedirectList,
}

/// A subshell in the shell grammar:
///
///     Subshell := "(" CompleteCommand ")" RedirectList
#[derive(Debug)]
pub struct Subshell {
    pub complete_command: CompleteCommand,
    pub redirs: RedirectList,
}

/// A list of redirections in the shell grammar:
///
///     RedirectList ::= /* Empty */
///                    |  "<" filename
///                    |  ">" filename
///                    | ">>" filename
#[derive(Debug)]
pub struct RedirectList {
    pub input: VecDeque<InputRedir>,
    pub output: VecDeque<OutputRedir>,
}

/// Describes how to perform input redirection.
#[derive(Debug, Clone)]
pub struct InputRedir {
    pub filename: String,
}

/// Describes how to perform output redirection.
#[derive(Debug, Clone)]
pub struct OutputRedir {
    pub filename: String,
    pub overwrite: bool,
}

/// Parses the incoming sequence of tokens.
pub fn parse(tokens: VecDeque<lexer::Token>) -> Result<CompleteCommand> {
    let mut parser = Parser::new(tokens);
    parser.run()
}

//
// Implementation of public types.
//

impl CompleteCommand {
    /// creates a new instance of CompleteCommand
    pub fn new() -> CompleteCommand {
        CompleteCommand {
            pipelines: VecDeque::<_>::new(),
        }
    }
}

impl Pipeline {
    /// creates a new instance of Pipeline
    pub fn new() -> Pipeline {
        Pipeline {
            commands: VecDeque::<_>::new(),
            sync: false,
        }
    }
}

impl SimpleCommand {
    /// creates a new instance of SimpleCommand
    pub fn new() -> SimpleCommand {
        SimpleCommand {
            arguments: VecDeque::<_>::new(),
            redirs: RedirectList::new(),
        }
    }
}

impl RedirectList {
    /// creates a new instance of RedirectList
    pub fn new() -> RedirectList {
        RedirectList {
            input: VecDeque::<_>::new(),
            output: VecDeque::<_>::new(),
        }
    }
}

//
// Parser implementation.
//

/// Parses a complete command.
struct Parser {
    tokens: VecDeque<lexer::Token>,
}

impl Parser {
    /// Creates a new parser instance
    fn new(tokens: VecDeque<lexer::Token>) -> Parser {
        Parser { tokens: tokens }
    }

    /// Runs the shell parser.
    fn run(self: &mut Self) -> Result<CompleteCommand> {
        let cc = self.parse_complete_command()?;
        let token = self.read()?;
        match token.kind {
            lexer::Kind::EndOfLine => (),
            _ => return Err(Error::new("expected EOL")),
        }
        Ok(cc)
    }

    /// Parses a complete command.
    fn parse_complete_command(self: &mut Self) -> Result<CompleteCommand> {
        let mut cc = CompleteCommand::new();
        loop {
            let mut pipeline = self.parse_pipeline()?;
            let token = self.read()?;
            match token.kind {
                lexer::Kind::Semicolon => {
                    pipeline.sync = true;
                    cc.pipelines.push_back(pipeline);
                }
                lexer::Kind::Ampersand => {
                    pipeline.sync = false;
                    cc.pipelines.push_back(pipeline);
                }
                lexer::Kind::EndOfLine => {
                    pipeline.sync = true;
                    cc.pipelines.push_back(pipeline);
                    self.unread(token);
                    break;
                }
                lexer::Kind::CloseBrace => {
                    pipeline.sync = true;
                    cc.pipelines.push_back(pipeline);
                    self.unread(token);
                    break;
                }
                _ => {
                    return Err(Error::new("expected ;&) or EOL"));
                }
            }
        }
        Ok(cc)
    }

    /// Parses a pipeline statement.
    fn parse_pipeline(self: &mut Self) -> Result<Pipeline> {
        let mut pipeline = Pipeline::new();
        loop {
            let command = self.parse_command()?;
            pipeline.commands.push_back(command);
            let token = self.read()?;
            match token.kind {
                lexer::Kind::Pipe => (),
                _ => {
                    self.unread(token);
                    break;
                }
            }
        }
        Ok(pipeline)
    }

    /// Parses a command statement.
    fn parse_command(self: &mut Self) -> Result<Command> {
        let token = self.read()?;
        match token.kind {
            lexer::Kind::OpenBrace => self.parse_subshell(),
            _ => {
                self.unread(token);
                self.parse_simple_command()
            }
        }
    }

    /// Parses a subshell command.
    fn parse_subshell(self: &mut Self) -> Result<Command> {
        // We have already consumed the '(' token
        let cc = self.parse_complete_command()?;
        let token = self.read()?;
        match token.kind {
            lexer::Kind::CloseBrace => (),
            _ => return Err(Error::new("expected ')' token")),
        }
        let redirs = self.parse_redirs()?;
        Ok(Command::Subshell(Subshell {
            complete_command: cc,
            redirs: redirs,
        }))
    }

    /// Parses a simple command.
    fn parse_simple_command(self: &mut Self) -> Result<Command> {
        let mut scmd = SimpleCommand::new();
        loop {
            let token = self.read()?;
            match token.kind {
                lexer::Kind::CommandOrArgument => {
                    scmd.arguments.push_back(token.value);
                }
                _ => {
                    self.unread(token);
                    break;
                }
            }
        }
        let redirs = self.parse_redirs()?;
        scmd.redirs = redirs;
        Ok(Command::SimpleCommand(scmd))
    }

    /// Parses a redirection.
    fn parse_redirs(self: &mut Self) -> Result<RedirectList> {
        let mut redirs = RedirectList::new();
        loop {
            let token = self.read()?;
            match token.kind {
                lexer::Kind::Minor => {
                    let value = self.read_command_or_argument_token()?;
                    redirs.input.push_front(InputRedir {
                        filename: value.value,
                    });
                }
                lexer::Kind::Major => {
                    let value = self.read_command_or_argument_token()?;
                    redirs.output.push_front(OutputRedir {
                        filename: value.value,
                        overwrite: true,
                    });
                }
                lexer::Kind::MajorMajor => {
                    let value = self.read_command_or_argument_token()?;
                    redirs.output.push_front(OutputRedir {
                        filename: value.value,
                        overwrite: false,
                    });
                }
                _ => {
                    self.unread(token);
                    break;
                }
            }
        }
        Ok(redirs)
    }

    /// Returns the next CommandOrArgument token or an error if
    /// we cannot find a token of this type in the input.
    fn read_command_or_argument_token(self: &mut Self) -> Result<lexer::Token> {
        let token = self.read()?;
        match token.kind {
            lexer::Kind::CommandOrArgument => Ok(token),
            _ => Err(Error::new("expected CommandOrArgument token")),
        }
    }

    /// Reads the next token in the input stream.
    fn read(self: &mut Self) -> Result<lexer::Token> {
        match self.tokens.pop_front() {
            None => Err(Error::new("unexpected end of input")),
            Some(token) => Ok(token),
        }
    }

    /// Unreads a token putting it back into the input stream.
    fn unread(self: &mut Self, token: lexer::Token) {
        self.tokens.push_front(token);
    }
}
