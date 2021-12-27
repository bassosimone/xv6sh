mod interp;
mod lexer;
mod model;
mod parser;
mod serializer;
mod translator;

use crate::model::{Error, Result};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let program = args[0].clone();
    let mut opts = getopts::Options::new();
    opts.optopt("c", "", "execute the given command line", "COMMANDS");
    let matches = match opts.parse(&args[1..]) {
        Err(_) => {
            eprintln!("usage: {} [-c COMMANDS]", program);
            std::process::exit(1);
        }
        Ok(m) => m,
    };
    if let Some(cmd) = matches.opt_str("c") {
        shrun(cmd);
        std::process::exit(0);
    }
    loop {
        match getcmd() {
            Err(_) => break,
            Ok(cmd) => shrun(cmd),
        }
    }
}

/// Interprets a single shell input line.
fn shrun(cmd: String) {
    match shrun_internal(cmd) {
        Ok(_) => (),
        Err(err) => eprintln!("{}", err),
    }
}

/// Interprets a single shell input line.
fn shrun_internal(cmd: String) -> Result<()> {
    let tokens = lexer::scan(cmd);
    println!("sh: tokens: {:?}", tokens);
    let tree = parser::parse(tokens)?;
    println!("sh: pass #1 tree: {:?}", tree);
    let loc = translator::translate(tree)?;
    println!("sh: pass #2 tree: {:?}", loc);
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
