use std::fmt::Write;

#[derive(Debug, Copy, Clone, PartialEq, Eq, ::enum_map::Enum)]
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
    EOF,
}

impl std::fmt::Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
// Could derive Copy as well, but I usually don't want to copy token, so I still require copies to
// be made explicitly by calling clone().
#[derive(Clone)]
pub struct Token<'a> {
    token_type: TokenType,
    lexeme: &'a [char],
    line: u32,
}

impl<'a> Token<'a> {
    pub fn new(token_type: TokenType, lexeme: &'a [char], line: u32) -> Self {
        Token {
            token_type,
            lexeme,
            line,
        }
    }

    pub fn get_token_type(&self) -> TokenType {
        self.token_type
    }

    pub fn get_lexeme(&self) -> &'a [char] {
        self.lexeme
    }

    pub fn get_line(&self) -> u32 {
        self.line
    }

    pub fn get_lexeme_string(&self) -> String {
        self.lexeme.iter().collect::<String>()
    }
}

impl<'a> std::fmt::Display for Token<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:12} '", self.token_type)?;
        for c in self.lexeme.iter() {
            f.write_char(*c)?;
        }
        f.write_char('\'')
    }
}
