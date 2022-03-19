use std::fmt::Write;

#[derive(Debug, Copy, Clone)]
pub enum TokenType {
    // Single-character tokens.
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Dot,
    Minus,
    Plus,
    Semicolon,
    Slash,
    Star,

    // One or two character tokens.
    Bang,
    BangEqual,
    Equal,
    EqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,

    // Literals.
    Identifier,
    String,
    Number,

    // KEYWORDS.
    And,
    Class,
    Else,
    False,
    Fun,
    For,
    If,
    Nil,
    Or,
    Print,
    Return,
    Super,
    This,
    True,
    Var,
    While,

    Error,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

pub struct Token<'a> {
    token_type: TokenType,
    lexme: &'a [char],
    line: u32,
}

impl<'a> Token<'a> {
    pub fn new(token_type: TokenType, lexme: &'a [char], line: u32) -> Self {
        Token {
            token_type,
            lexme,
            line,
        }
    }

    pub fn get_token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn get_lexme(&self) -> &'a [char] {
        self.lexme
    }

    pub fn get_line(&self) -> u32 {
        self.line
    }
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:12} '", self.token_type)?;
        for c in self.lexme.iter() {
            f.write_char(*c)?;
        }
        f.write_char('\'')
    }
}
