#[macro_use]
extern crate text_io;

mod lexer;
mod parser;

fn main() {
    loop {
        let cmd = getcmd();
        if cmd == "" {
            break;
        }
        if cmd.starts_with("cd ") {
            // Chdir must be called by the parent, not the child.
            // TODO: this code should be interpreted _after_ correct parsing.
            let directory = &cmd[3..];
            chdir(directory);
            continue;
        }
        let tokens = lexer::scan(cmd);
        println!("sh: tokens: {:?}", tokens);
        let cc = parser::parse(tokens);
        println!("sh: parse tree: {:?}", cc);
    }
}

/// Changes the current working directory.
fn chdir(directory: &str) -> () {
    let result = std::env::set_current_dir(directory);
    match result {
        Err(err) => {
            eprintln!("cd: {}", err);
        }
        Ok(_) => {}
    }
}

/// Reads a command from the standard input.
fn getcmd() -> String {
    use std::io::Write;
    print!("$ ");
    std::io::stdout().flush().unwrap();
    let line: String = read!("{}\n");
    line
}
