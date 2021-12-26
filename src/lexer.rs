//! Scanner implementation.

use std::collections::VecDeque;

/// Kind of a scanned token.
#[derive(Debug)]
pub enum Kind {
    Pipe,
    OpenBrace,
    CloseBrace,
    Semicolon,
    Ampersand,
    Minor,
    Major,
    MajorMajor,
    CommandOrArgument,
    EndOfLine,
}

/// Scanned token.
#[derive(Debug)]
pub struct Token {
    /// The kind of the token.
    pub kind: Kind,

    /// The token's value.
    pub value: String,
}

/// Scans the command line.
pub fn scan(cmdline: String) -> VecDeque<Token> {
    let mut lexer = Lexer::new(cmdline);
    lexer.run();
    lexer.r
}

/// Lexer for the command line.
struct Lexer {
    /// buffer for constructing CommandOrArgument tokens.
    buff: String,

    /// whether we're inside a CommandOrArgment token.
    inside: bool,

    /// input contains the input.
    input: VecDeque<char>,

    /// contains the stream of tokens.
    r: VecDeque<Token>,
}

impl Lexer {
    /// converts the input to a deque.
    fn to_deque(input: String) -> VecDeque<char> {
        let mut r = VecDeque::<char>::new();
        for c in input.chars() {
            r.push_back(c);
        }
        r
    }

    /// creates a new lexer instance.
    fn new(input: String) -> Lexer {
        Lexer {
            buff: String::new(),
            inside: false,
            input: Self::to_deque(input),
            r: VecDeque::<Token>::new(),
        }
    }

    /// runs the scanner.
    fn run(self: &mut Self) {
        loop {
            if let Some(c) = self.peek() {
                self.advance();
                let end_of_line = self.process_current(c);
                if end_of_line {
                    break;
                }
            } else {
                break;
            }
        }
        self.leave_and_push_back(Kind::EndOfLine);
    }

    /// processes the current char of the input stream and, if needed,
    /// also processes subsequent chars. Returns true whether we've
    /// reached the end of the input, false otherwise.
    fn process_current(self: &mut Self, c: char) -> bool {
        let mut at_eol = false;
        if c == ' ' || c == '\t' {
            self.leave();
        } else if c == '|' {
            self.leave_and_push_back(Kind::Pipe);
        } else if c == '(' {
            self.leave_and_push_back(Kind::OpenBrace);
        } else if c == ')' {
            self.leave_and_push_back(Kind::CloseBrace);
        } else if c == ';' {
            self.leave_and_push_back(Kind::Semicolon);
        } else if c == '&' {
            self.leave_and_push_back(Kind::Ampersand);
        } else if c == '<' {
            self.leave_and_push_back(Kind::Minor);
        } else if c == '>' {
            if let Some(c) = self.peek() {
                if c == '>' {
                    self.advance();
                    self.leave_and_push_back(Kind::MajorMajor);
                } else {
                    self.leave_and_push_back(Kind::Major);
                }
            } else {
                self.leave_and_push_back(Kind::Major);
                at_eol = true;
            }
        } else {
            self.enter_or_persist(c);
        }
        return at_eol;
    }

    // TODO: rewrite using read/unread instead of peek/avance

    /// peek returns the next character in input without
    /// advancing the input iterator. Returns None in case
    /// we've reached the end of line (EOL).
    fn peek(self: &mut Self) -> Option<char> {
        match self.input.front() {
            None => None,
            Some(c) => Some(*c),
        }
    }

    /// advance discards the current input character. You
    /// MUST call this function after you've called peek and
    /// you already know we've not reached EOL.
    fn advance(self: &mut Self) {
        let _ = self.input.pop_front();
    }

    /// enters or continues to be inside a CommandOrArgument token
    /// and appends the current char to the token's value.
    fn enter_or_persist(self: &mut Self, c: char) {
        self.inside = true;
        self.buff.push(c);
    }

    /// possibly leaves the current token and then pushes back
    /// the given token into the token stream.
    fn leave_and_push_back(self: &mut Self, kind: Kind) {
        self.leave_and_push_back_string(kind, String::new());
    }

    /// possibly leaves the current token and then pushes back the given
    /// token into the token stream using the given value.
    fn leave_and_push_back_string(self: &mut Self, kind: Kind, value: String) {
        self.leave();
        self.r.push_back(Token {
            kind: kind,
            value: value,
        });
    }

    /// called when we stop being inside a CommandOrArgument to
    /// gracefully leave the CommandOrArgument state.
    fn leave(self: &mut Self) -> () {
        self.inside = false;
        if self.buff != "" {
            self.r.push_back(Token {
                kind: Kind::CommandOrArgument,
                value: self.buff.clone(),
            });
            self.buff.clear();
        }
    }
}
