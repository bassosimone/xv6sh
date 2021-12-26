#[macro_use]
extern crate text_io;

mod lexer;

fn main() {
    loop {
        let cmd = getcmd();
        if cmd == "" {
            break;
        }
        if cmd.starts_with("cd ") {
            // Chdir must be called by the parent, not the child.
            let directory = &cmd[3..];
            chdir(directory);
            continue;
        }
        let tokens = lexer::scan(cmd);
        for tok in tokens {
            println!("- {:?}", tok);
        }
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
