mod compiler;
mod lexer;
mod model;
mod parser;
mod serializer;

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
    let tokens = lexer::scan(cmd);
    println!("sh: tokens: {:?}", tokens);
    let cc = parser::parse(tokens);
    match cc {
        Err(_) => (),
        Ok(cc) => {
            println!("sh: parse tree: {:?}", cc);
            let bytecode = compiler::compile(cc);
            println!("sh: bytecode {:?}", bytecode);
        }
    }
}

/// Reads a command from the standard input.
fn getcmd() -> Result<String, std::io::Error> {
    use std::io::BufRead;
    use std::io::Write;
    print!("$ ");
    std::io::stdout().flush().unwrap();
    let stdin = std::io::stdin();
    let lines = stdin.lock().lines().next();
    match lines {
        Some(line) => line,
        None => Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "EOF",
        )),
    }
}
