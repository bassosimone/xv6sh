//! Translates the syntax tree into an executable syntax tree
//! that the interpreter will then interpret.

use crate::model::{Error, Result};
use crate::parser::{
    Command, CompleteCommand, InputRedir, OutputRedir, Pipeline, SimpleCommand, Subshell,
};
use crate::serializer;
use std::collections::VecDeque;

/// Contains a list of commands to run serially.
#[derive(Debug)]
pub struct ListOfCommands {
    pub pipelines: VecDeque<CompoundSerialCommand>,
}

/// A command that is run serially.
#[derive(Debug)]
pub enum CompoundSerialCommand {
    SingleCommand(SingleCommand),
    PipelinedCommands(PipelinedCommands),
}

/// A single, standalone command.
#[derive(Debug)]
pub struct SingleCommand {
    pub arguments: VecDeque<String>,
    pub input: Option<InputRedir>,
    pub output: Option<OutputRedir>,
    pub sync: bool,
}

/// A pipeline consisting of a SourceCommand, zero or more
/// FilterCommands and a SinkCommand.
#[derive(Debug)]
pub struct PipelinedCommands {
    pub source: SourceCommand,
    pub filters: VecDeque<FilterCommand>,
    pub sink: SinkCommand,
    pub sync: bool,
}

/// The source command of a pipeline.
#[derive(Debug)]
pub struct SourceCommand {
    pub arguments: VecDeque<String>,
    pub input: Option<InputRedir>,
}

/// A filter command in the middle of a pipeline.
#[derive(Debug)]
pub struct FilterCommand {
    pub arguments: VecDeque<String>,
}

/// The sink command of a pipeline.
#[derive(Debug)]
pub struct SinkCommand {
    pub arguments: VecDeque<String>,
    pub output: Option<OutputRedir>,
}

/// Translates the syntax tree to make it interpretable.
pub fn translate(cc: CompleteCommand, verbose: bool) -> Result<ListOfCommands> {
    let translator = Translator::new(verbose);
    translator.complete_command(cc)
}

//
// Public implementation
//

impl ListOfCommands {
    /// Creates a new list of commands.
    pub fn new() -> ListOfCommands {
        ListOfCommands {
            pipelines: VecDeque::<_>::new(),
        }
    }
}

impl SingleCommand {
    /// Creates a new single command.
    pub fn new() -> SingleCommand {
        SingleCommand {
            arguments: VecDeque::<_>::new(),
            input: None,
            output: None,
            sync: false,
        }
    }
}

impl PipelinedCommands {
    /// Creates a new pipelined commands instance.
    pub fn new() -> PipelinedCommands {
        PipelinedCommands {
            source: SourceCommand::new(),
            filters: VecDeque::<_>::new(),
            sink: SinkCommand::new(),
            sync: false,
        }
    }
}

impl SourceCommand {
    /// Creates a new source command.
    pub fn new() -> SourceCommand {
        SourceCommand {
            arguments: VecDeque::<_>::new(),
            input: None,
        }
    }
}

impl FilterCommand {
    /// Creates a new filter command.
    pub fn new() -> FilterCommand {
        FilterCommand {
            arguments: VecDeque::<_>::new(),
        }
    }
}

impl SinkCommand {
    /// Creates a new sink command.
    pub fn new() -> SinkCommand {
        SinkCommand {
            arguments: VecDeque::<_>::new(),
            output: None,
        }
    }
}

//
// Translator implementation
//

/// The translator itself.
struct Translator {
    verbose: bool,
}

impl Translator {
    /// creates a new translator
    fn new(verbose: bool) -> Translator {
        Translator { verbose: verbose }
    }

    /// visits each pipeline inside the complete command.
    fn complete_command(self: &Self, input: CompleteCommand) -> Result<ListOfCommands> {
        let mut output = ListOfCommands::new();
        let mut pipelines = input.pipelines;
        loop {
            match pipelines.pop_front() {
                None => break,
                Some(pipeline) => {
                    if pipeline.commands.len() < 1 {
                        continue; // skip empty pipelines
                    }
                    let pipeline = self.pipeline(pipeline)?;
                    output.pipelines.push_back(pipeline);
                }
            }
        }
        Ok(output)
    }

    /// visits each command inside the pipeline.
    fn pipeline(self: &Self, input: Pipeline) -> Result<CompoundSerialCommand> {
        let mut intermediate = VecDeque::<SimpleCommand>::new();
        let sync = input.sync;
        let mut input = input.commands;
        loop {
            match input.pop_front() {
                None => break,
                Some(cmd) => {
                    let scmd = self.command(cmd)?;
                    intermediate.push_back(scmd);
                }
            }
        }
        if intermediate.len() < 1 {
            return Err(Error::new("no intermediate commands"));
        }
        if intermediate.len() == 1 {
            let f = intermediate.pop_front().unwrap(); // cannot fail
            return self.single_command(f, sync);
        }
        self.pipelined_commands(intermediate, sync)
    }

    /// produces a single command instance
    fn single_command(
        self: &Self,
        input: SimpleCommand,
        sync: bool,
    ) -> Result<CompoundSerialCommand> {
        let mut output = SingleCommand::new();
        output.arguments = input.arguments;
        output.sync = sync;
        if input.redirs.input.len() > 1 {
            return Err(Error::new("more than one input redirection"));
        }
        if input.redirs.input.len() == 1 {
            output.input = Some(input.redirs.input[0].clone());
        }
        if input.redirs.output.len() > 1 {
            return Err(Error::new("more than one output redirection"));
        }
        if input.redirs.output.len() == 1 {
            output.output = Some(input.redirs.output[0].clone());
        }
        Ok(CompoundSerialCommand::SingleCommand(output))
    }

    /// produces pipelined commands
    fn pipelined_commands(
        self: &Self,
        mut input: VecDeque<SimpleCommand>,
        sync: bool,
    ) -> Result<CompoundSerialCommand> {
        let mut output = PipelinedCommands::new();
        output.sync = sync;
        output.source = self.new_source(&mut input)?;
        output.filters = self.new_filters(&mut input)?;
        output.sink = self.new_sink(&mut input)?;
        Ok(CompoundSerialCommand::PipelinedCommands(output))
    }

    /// Helper for pipelined_commands
    fn new_source(self: &Self, input: &mut VecDeque<SimpleCommand>) -> Result<SourceCommand> {
        let mut output = SourceCommand::new();
        match input.pop_front() {
            None => Err(Error::new("unexpected empty deque")),
            Some(item) => {
                output.arguments = item.arguments;
                if item.redirs.input.len() > 1 {
                    return Err(Error::new("more than one input redirection"));
                }
                if item.redirs.input.len() == 1 {
                    output.input = Some(item.redirs.input[0].clone());
                }
                if item.redirs.output.len() > 0 {
                    return Err(Error::new("output redirection for pipeline source"));
                }
                Ok(output)
            }
        }
    }

    /// Helper for pipelined_commands
    fn new_filters(
        self: &Self,
        input: &mut VecDeque<SimpleCommand>,
    ) -> Result<VecDeque<FilterCommand>> {
        let mut output = VecDeque::<FilterCommand>::new();
        while input.len() > 1 {
            // note: the last element is the sink
            let e = input.pop_front().unwrap(); // cannot fail
            let mut filter = FilterCommand::new();
            filter.arguments = e.arguments;
            if e.redirs.input.len() > 0 {
                return Err(Error::new("input redirection for pipeline filter"));
            }
            if e.redirs.output.len() > 0 {
                return Err(Error::new("output redirection for pipeline filter"));
            }
            output.push_back(filter);
        }
        Ok(output)
    }

    /// Helper for pipelined_commands
    fn new_sink(self: &Self, input: &mut VecDeque<SimpleCommand>) -> Result<SinkCommand> {
        let mut output = SinkCommand::new();
        match input.pop_front() {
            None => Err(Error::new("unexpected empty deque")),
            Some(item) => {
                output.arguments = item.arguments;
                if item.redirs.input.len() > 0 {
                    return Err(Error::new("input redirection for pipeline sink"));
                }
                if item.redirs.output.len() > 1 {
                    return Err(Error::new("more than one output redirection"));
                }
                if item.redirs.output.len() == 1 {
                    output.output = Some(item.redirs.output[0].clone());
                }
                Ok(output)
            }
        }
    }

    /// visits a specific command
    fn command(self: &Self, input: Command) -> Result<SimpleCommand> {
        match input {
            Command::SimpleCommand(cmd) => Ok(cmd),
            Command::Subshell(ss) => self.subshell(ss),
        }
    }

    /// visits a subshell
    fn subshell(self: &Self, input: Subshell) -> Result<SimpleCommand> {
        let mut scmd = SimpleCommand {
            arguments: VecDeque::<_>::new(),
            redirs: input.redirs,
        };
        let exe = Self::get_current_exe()?;
        scmd.arguments.push_back(exe);
        if self.verbose {
            scmd.arguments.push_back(String::from("-x"))
        }
        scmd.arguments.push_back(String::from("-c"));
        let serialized = serializer::serialize(input.complete_command)?;
        scmd.arguments.push_back(serialized);
        Ok(scmd)
    }

    /// Helper function to obtain the current exe.
    fn get_current_exe() -> Result<String> {
        match std::env::current_exe() {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok(pb) => match pb.to_str() {
                None => Err(Error::new("unicode decode error")),
                Some(path) => Ok(String::from(path)),
            },
        }
    }
}
