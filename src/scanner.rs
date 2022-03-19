use lazy_static::lazy_static;

use crate::tokens::{Token, TokenType};

// Turn a &str in a Vec<char>.
macro_rules! chars {
    ($str:literal) => {
        $str.to_string().chars().collect::<Vec<char>>()
    };
}

// This is a hack a to get error messages in form of &'static [char] and should be replaced by some better means.
lazy_static! {
    // Used for error messages.
    static ref UNEXPECTED_CHAR: Vec<char> = chars!("Unexpected character.");
    static ref UNTERMINATED_STRING: Vec<char> = chars!("Unterminated string.");

    // Needed for the trie used to check for keywords.
    static ref AR: Vec<char> = chars!("ar");
    static ref ETURN: Vec<char> = chars!("eturn");
    static ref F: Vec<char> = chars!("f");
    static ref HILE: Vec<char> = chars!("hile");
    static ref IL: Vec<char> = chars!("il");
    static ref IS: Vec<char> = chars!("is");
    static ref LASS: Vec<char> = chars!("lass");
    static ref LSE: Vec<char> = chars!("lse");
    static ref N: Vec<char> = chars!("n");
    static ref ND: Vec<char> = chars!("nd");
    static ref R: Vec<char> = chars!("r");
    static ref RINT: Vec<char> = chars!("rint");
    static ref UE: Vec<char> = chars!("ue");
    static ref UPER: Vec<char> = chars!("uper");
}

/// The Scanner is used to parse the input in form of a &[char] into a token stream.
/// This is done lazily by using an iterator.
/// Error in the input slice are reported as TokenType::Error inline in the iterator sequence.
pub struct Scanner<'a> {
    scanner: ScannerImpl<'a>,
}

impl<'a> Scanner<'a> {
    /// Construct a Scanner from a char slice.
    pub fn new(source: &'a [char]) -> Self {
        Scanner {
            scanner: ScannerImpl::new(source),
        }
    }

    /// Parse the given input sequence lazily into a sequence of tokens.
    pub fn parse(self) -> impl Iterator<Item = Token<'a>> {
        self.scanner
    }
}

struct ScannerImpl<'a> {
    source: &'a [char],
    start: usize,
    current: usize,
    line: u32,
}

impl<'a> ScannerImpl<'a> {
    fn new(source: &'a [char]) -> Self {
        ScannerImpl {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }

    fn scan_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return None;
        }

        let c = self.advance();

        if is_alpha(c) {
            return Some(self.identifier());
        }

        if c.is_digit(10) {
            return Some(self.number());
        }

        let token = match c {
            '(' => self.make_token(TokenType::LeftParen),
            ')' => self.make_token(TokenType::RightParen),
            '{' => self.make_token(TokenType::LeftBrace),
            '}' => self.make_token(TokenType::RightBrace),
            ';' => self.make_token(TokenType::Semicolon),
            ',' => self.make_token(TokenType::Comma),
            '.' => self.make_token(TokenType::Dot),
            '-' => self.make_token(TokenType::Minus),
            '+' => self.make_token(TokenType::Plus),
            '/' => self.make_token(TokenType::Slash),
            '*' => self.make_token(TokenType::Star),
            '!' => {
                let tt = if self.matches('=') {
                    TokenType::BangEqual
                } else {
                    TokenType::Bang
                };
                self.make_token(tt)
            }
            '=' => {
                let tt = if self.matches('=') {
                    TokenType::EqualEqual
                } else {
                    TokenType::Equal
                };
                self.make_token(tt)
            }
            '<' => {
                let tt = if self.matches('=') {
                    TokenType::LessEqual
                } else {
                    TokenType::Less
                };
                self.make_token(tt)
            }
            '>' => {
                let tt = if self.matches('=') {
                    TokenType::GreaterEqual
                } else {
                    TokenType::Greater
                };
                self.make_token(tt)
            }
            '"' => self.string(),
            _ => self.error_token(UNEXPECTED_CHAR.as_slice()),
        };

        Some(token)
    }

    fn identifier(&mut self) -> Token<'a> {
        while is_alpha(self.peek()) || self.peek().is_digit(10) {
            self.advance();
        }

        let ident_type = self.identifier_type();
        self.make_token(ident_type)
    }

    fn identifier_type(&mut self) -> TokenType {
        match self.source[self.start] {
            'a' => self.check_keyword(1, ND.as_slice(), TokenType::And),
            'c' => self.check_keyword(1, LASS.as_slice(), TokenType::Class),
            'e' => self.check_keyword(1, LSE.as_slice(), TokenType::Else),
            'f' => {
                if self.current - self.start > 1 {
                    match self.source[self.start + 1] {
                        'a' => self.check_keyword(2, LSE.as_slice(), TokenType::False),
                        'o' => self.check_keyword(2, R.as_slice(), TokenType::For),
                        'u' => self.check_keyword(2, N.as_slice(), TokenType::Fun),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            'i' => self.check_keyword(1, F.as_slice(), TokenType::If),
            'n' => self.check_keyword(1, IL.as_slice(), TokenType::Nil),
            'o' => self.check_keyword(1, R.as_slice(), TokenType::Or),
            'p' => self.check_keyword(1, RINT.as_slice(), TokenType::Print),
            'r' => self.check_keyword(1, ETURN.as_slice(), TokenType::Return),
            's' => self.check_keyword(1, UPER.as_slice(), TokenType::Super),
            't' => {
                if self.current - self.start > 1 {
                    match self.source[self.start + 1] {
                        'h' => self.check_keyword(2, IS.as_slice(), TokenType::This),
                        'r' => self.check_keyword(2, UE.as_slice(), TokenType::True),
                        _ => TokenType::Identifier,
                    }
                } else {
                    TokenType::Identifier
                }
            }
            'v' => self.check_keyword(1, AR.as_slice(), TokenType::Var),
            'w' => self.check_keyword(1, HILE.as_slice(), TokenType::While),
            _ => TokenType::Identifier,
        }
    }

    fn check_keyword(
        &self,
        start: usize,
        rest: &'static [char],
        token_type: TokenType,
    ) -> TokenType {
        if &self.source[(self.start + start)..self.current] == rest {
            token_type
        } else {
            TokenType::Identifier
        }
    }

    fn string(&mut self) -> Token<'a> {
        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
            }

            self.advance();
        }

        if self.is_at_end() {
            self.error_token(UNTERMINATED_STRING.as_slice())
        } else {
            self.advance();
            self.make_token(TokenType::String)
        }
    }

    fn number(&mut self) -> Token<'a> {
        while self.peek().is_digit(10) {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_digit(10) {
            self.advance();
        }

        while self.peek().is_digit(10) {
            self.advance();
        }

        self.make_token(TokenType::Number)
    }

    fn skip_whitespace(&mut self) {
        loop {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.current += 1;
                }
                '\n' => {
                    self.line += 1;
                    self.current += 1;
                }
                '/' => {
                    if self.peek_next() == '/' {
                        while self.peek() != '\n' && !self.is_at_end() {
                            self.advance();
                        }
                    } else {
                        return;
                    }
                }
                _ => return,
            }
        }
    }

    fn matches(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.peek() != expected {
            false
        } else {
            self.advance();
            true
        }
    }

    fn advance(&mut self) -> char {
        let c = self.peek();
        self.current += 1;
        c
    }

    fn peek(&self) -> char {
        self.source[self.current]
    }

    fn peek_next(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == self.source.len()
    }

    fn make_token(&self, token_type: TokenType) -> Token<'a> {
        let lexme = &self.source[self.start..self.current];
        Token::new(token_type, lexme, self.line)
    }

    fn error_token(&self, message: &'static [char]) -> Token<'a> {
        Token::new(TokenType::Error, message, self.line)
    }
}

impl<'a> Iterator for ScannerImpl<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> std::option::Option<<Self as std::iter::Iterator>::Item> {
        self.scan_token()
    }
}

fn is_alpha(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}
