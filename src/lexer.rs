/// A token inside the command line.
#[derive(Debug)]
pub enum Token {
    Pipe,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Ampersand,
    Minor,
    Major,
    CmdOrArg(String),
    EndOfLine,
}

/// Scans the command line.
pub fn scan(cmdline: String) -> Vec<Token> {
    let mut lexer = Lexer::new();
    lexer.run(cmdline)
}

/// Lexer for the command line.
struct Lexer {
    buffer: String,
    inside: bool,
}

impl Lexer {
    /// creates a new lexer instance.
    fn new() -> Lexer {
        Lexer {
            buffer: "".to_owned(),
            inside: false,
        }
    }

    /// runs the scanner.
    fn run(self: &mut Self, input: String) -> Vec<Token> {
        let mut r = Vec::<Token>::new();
        for c in input.chars() {
            if c == ' ' || c == '\t' {
                self.leave(&mut r);
                continue;
            }
            if c == '|' {
                self.leave(&mut r);
                r.push(Token::Pipe);
                continue;
            }
            if c == '(' {
                self.leave(&mut r);
                r.push(Token::OpenBrace);
                continue;
            }
            if c == ')' {
                self.leave(&mut r);
                r.push(Token::CloseBrace);
                continue;
            }
            if c == ';' {
                self.leave(&mut r);
                r.push(Token::Semicolon);
                continue;
            }
            if c == '&' {
                self.leave(&mut r);
                r.push(Token::Ampersand);
                continue;
            }
            if c == '<' {
                self.leave(&mut r);
                r.push(Token::Minor);
                continue;
            }
            if c == '>' {
                self.leave(&mut r);
                r.push(Token::Major);
                continue;
            }
            self.inside = true;
            self.buffer.push(c);
        }
        self.leave(&mut r);
        r.push(Token::EndOfLine);
        r
    }

    /// called when we stop being inside a command or argument.
    fn leave(self: &mut Self, toks: &mut Vec<Token>) -> () {
        self.inside = false;
        if self.buffer != "" {
            toks.push(Token::CmdOrArg(self.buffer.clone()));
            self.buffer.clear();
        }
    }
}