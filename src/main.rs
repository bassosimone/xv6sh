//! Unix v6-like shell written in rust.

mod background;
mod interp;
mod lexer;
mod model;
mod parser;
mod serializer;
mod translator;

use crate::model::{Error, Result};

/// Main function.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();
    let mut opts = getopts::Options::new();
    opts.optopt("c", "", "execute the given command line", "COMMANDS");
    opts.optopt("", "stage", "stop processing at the given stage", "STAGE");
    opts.optflag("x", "", "turn debugging on");
    let matches = match opts.parse(&args[1..]) {
        Err(_) => {
            eprintln!(
                "usage: {} [--stage scan|parse|plan|run] [-x] [-c COMMANDS]",
                program
            );
            std::process::exit(1);
        }
        Ok(m) => m,
    };
    if matches.opt_present("x") {
        interp::set_verbose();
    }
    let stage = matches.opt_str("stage").or(Some(String::new())).unwrap();
    if let Some(cmd) = matches.opt_str("c") {
        shrunx(cmd, &stage);
        std::process::exit(0);
    }
    loop {
        match getcmd() {
            Err(_) => break,
            Ok(cmd) => shrunx(cmd, &stage),
        }
    }
}

/// Interprets a single shell input line.
fn shrunx(cmd: String, stage: &String) {
    match shrun(cmd, stage) {
        Ok(_) => (),
        Err(err) => eprintln!("xv6sh: error: {}", err),
    }
}

/// Interprets a single shell input line.
fn shrun(cmd: String, stage: &String) -> Result<()> {
    background::collect(); // ensure we don't leave zombies around
    let tokens = lexer::scan(cmd);
    if stage == "scan" {
        println!("{:#?}", tokens);
        return Ok(());
    }
    let tree = parser::parse(tokens)?;
    if stage == "parse" {
        println!("{:#?}", tree);
        return Ok(());
    }
    let loc = translator::translate(tree)?;
    if stage == "plan" {
        println!("{:#?}", loc);
        return Ok(());
    }
    interp::interpret(loc)
}

/// Reads a command from the standard input.
fn getcmd() -> Result<String> {
    use std::io::BufRead;
    use std::io::Write;
    print!("$ ");
    std::io::stdout().flush().unwrap();
    let stdin = std::io::stdin();
    let lines = stdin.lock().lines().next();
    match lines {
        Some(line) => match line {
            Err(err) => Err(Error::new(&err.to_string())),
            Ok(line) => Ok(line),
        },
        None => Err(Error::new("EOF")),
    }
}
