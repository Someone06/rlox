use crate::tokens::{Token, TokenType};

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => {
        $sub
    };
}

// Count the number of token at compile time.
// See: https://danielkeep.github.io/tlborm/book/blk-counting.html#slice-length
macro_rules! count_tts {
    ($($tts:tt)*) => {
        <[()]>::len(&[$(replace_expr!($tts ())),*])
    };
}

// This macro constructs a char array with a given name from a list of chars.
// It would be a lot nicer, if we could somehow directly turn a string into a char array.
// However, currently the only way to deconstruct a String (or str) to a sequence of char is byi
// using String::chars() which is not const.
// So until there is a better way to turn a string into a char, this ugly hack will stay in place.
macro_rules! chars {
   ($name:ident $($c:literal)+) => {
      const $name: [char; count_tts!($($c)+)] = [ $($c,)+ ];
    };
}

// Error messages.
chars! {UNEXPECTED_CHAR 'U' 'n' 'e' 'x' 'p' 'e' 'c' 't' 'e' 'd' ' ' 'c' 'h' 'a' 'r' 'a' 'c' 't' 'e' 'r' '.'}
chars! {UNTERMINATED_STRING 'U' 'n' 't' 'e' 'r' 'm' 'i' 'n' 'a' 't' 'e' 'd' ' ' 's' 't' 'r' 'i' 'n' 'g' '.'}

// Used to check for keywords.
chars! {AR 'a' 'r'}
chars! {ETURN 'e' 't' 'u' 'r' 'n'}
chars! {F 'f'}
chars! {HILE 'h' 'i' 'l' 'e'}
chars! {IL 'i' 'l'}
chars! {IS 'i' 's'}
chars! {LASS 'l' 'a' 's' 's'}
chars! {LSE 'l' 's' 'e'}
chars! {N 'n'}
chars! {ND 'n' 'd'}
chars! {R 'r'}
chars! {RINT 'r' 'i' 'n' 't'}
chars! {UE 'u' 'e'}
chars! {UPER 'u' 'p' 'e' 'r'}

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
    returned_eof: bool,
}

impl<'a> ScannerImpl<'a> {
    fn new(source: &'a [char]) -> Self {
        ScannerImpl {
            source,
            start: 0,
            current: 0,
            line: 1,
            returned_eof: false,
        }
    }

    fn scan_token(&mut self) -> Option<Token<'a>> {
        self.skip_whitespace();
        self.start = self.current;

        if self.is_at_end() {
            return if self.returned_eof {
                None
            } else {
                self.returned_eof = true;
                Some(self.make_token(TokenType::Eof))
            };
        }

        let c = self.advance();

        if is_alpha(c) {
            return Some(self.identifier());
        }

        if c.is_ascii_digit() {
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
        while !self.is_at_end() && (is_alpha(self.peek()) || self.peek().is_ascii_digit()) {
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
        while !self.is_at_end() && self.peek() != '"' {
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
        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        if !self.is_at_end() && self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();
        }

        while !self.is_at_end() && self.peek().is_ascii_digit() {
            self.advance();
        }

        self.make_token(TokenType::Number)
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
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
                        while !self.is_at_end() && self.peek() != '\n' {
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
        if self.current + 1 < self.source.len() {
            self.source[self.current + 1]
        } else {
            '\0'
        }
    }

    fn is_at_end(&self) -> bool {
        self.current == self.source.len()
    }

    fn make_token(&self, token_type: TokenType) -> Token<'a> {
        let lexeme = &self.source[self.start..self.current];
        Token::new(token_type, lexeme, self.line)
    }

    fn error_token(&self, message: &'static [char]) -> Token<'a> {
        Token::new(TokenType::Error, message, self.line)
    }
}

impl<'a> Iterator for ScannerImpl<'a> {
    type Item = Token<'a>;
    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.scan_token()
    }
}

// Underscores are allowed anywhere in identifiers.
fn is_alpha(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

#[cfg(test)]
mod tests {
    use crate::scanner::Scanner;
    use crate::tokens::{Token, TokenType};

    macro_rules! chars {
        ($input: expr) => {
            $input.to_string().chars().collect::<Vec<char>>()
        };
    }

    macro_rules! scan {
        ($v:expr) => {
            Scanner::new($v.as_slice()).parse().collect::<Vec<Token>>()
        };
    }

    macro_rules! tt {
        ($v:ident) => {
            $v.iter()
                .map(|t| t.get_token_type())
                .collect::<Vec<TokenType>>()
        };
    }

    macro_rules! lexemes {
        ($v:ident) => {
            $v.iter()
                .filter(|t| t.get_token_type() != TokenType::Eof)
                .map(|t| t.get_lexeme())
                .map(|l| l.iter().collect::<String>())
                .collect::<Vec<String>>()
        };
    }

    #[test]
    fn punctuation() {
        let input = chars!("(){};,.-+/*!!====<<=>>=");
        let result = scan!(input);

        let expected_types = vec![
            TokenType::LeftParen,
            TokenType::RightParen,
            TokenType::LeftBrace,
            TokenType::RightBrace,
            TokenType::Semicolon,
            TokenType::Comma,
            TokenType::Dot,
            TokenType::Minus,
            TokenType::Plus,
            TokenType::Slash,
            TokenType::Star,
            TokenType::Bang,
            TokenType::BangEqual,
            TokenType::EqualEqual,
            TokenType::Equal,
            TokenType::Less,
            TokenType::LessEqual,
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Eof,
        ];
        assert_eq!(tt!(result), expected_types);
    }

    #[test]
    fn numbers() {
        let expected = vec!["1", "2", "3.4", "5.6", "123.346", "0.034"];
        let input = chars!(expected.join(" "));
        let result = scan!(input);
        assert_eq!(result.len(), expected.len() + 1);
        assert!(tt!(result)
            .iter()
            .filter(|tt| *tt != &TokenType::Eof)
            .all(|t| t.eq(&TokenType::Number)));
        assert_eq!(lexemes!(result), expected);
    }

    #[test]
    fn keywords() {
        let keyword = vec![
            "and", "class", "else", "false", "for", "fun", "if", "nil", "or", "print", "return",
            "super", "this", "true", "var", "while",
        ];

        let tokens = vec![
            TokenType::And,
            TokenType::Class,
            TokenType::Else,
            TokenType::False,
            TokenType::For,
            TokenType::Fun,
            TokenType::If,
            TokenType::Nil,
            TokenType::Or,
            TokenType::Print,
            TokenType::Return,
            TokenType::Super,
            TokenType::This,
            TokenType::True,
            TokenType::Var,
            TokenType::While,
            TokenType::Eof,
        ];

        let input = chars!(keyword.join(" "));
        let result = scan!(input);
        assert_eq!(result.len(), keyword.len() + 1);
        assert_eq!(tt!(result), tokens);
    }

    #[test]
    fn identifiers() {
        let input = chars!("iff suuper fun_ H3110");
        let expected = vec!["iff", "suuper", "fun_", "H3110"];
        let result = scan!(input);

        assert_eq!(result.len(), expected.len() + 1);
        assert!(tt!(result)
            .iter()
            .filter(|tt| *tt != &TokenType::Eof)
            .all(|t| t.eq(&TokenType::Identifier)));

        assert_eq!(lexemes!(result), expected);
    }

    #[test]
    fn strings() {
        let input = chars!("\"if\" \"super\" \"h3110\"");
        let expected = vec!["\"if\"", "\"super\"", "\"h3110\""];
        let result = scan!(input);

        assert_eq!(result.len(), expected.len() + 1);
        assert!(tt!(result)
            .iter()
            .filter(|tt| *tt != &TokenType::Eof)
            .all(|t| t.eq(&TokenType::String)));

        assert_eq!(lexemes!(result), expected);
    }

    #[test]
    fn not_terminated_string() {
        let input = chars!("\"if");
        let result = scan!(input);

        assert_eq!(result.len(), 2);
        assert_eq![result[0].get_token_type(), TokenType::Error];
        assert_eq![result[1].get_token_type(), TokenType::Eof];
    }

    #[test]
    fn unexpected_character() {
        let input = chars!("if$");
        let result = scan!(input);

        assert_eq!(result.len(), 3);
        assert_eq![result[0].get_token_type(), TokenType::If];
        assert_eq![result[1].get_token_type(), TokenType::Error];
        assert_eq![result[2].get_token_type(), TokenType::Eof];
    }

    #[test]
    fn whitespace() {
        let input = chars!("if\t(\r\ntrue\n\n\t )\n");
        let result = scan!(input);

        let expected = vec![
            TokenType::If,
            TokenType::LeftParen,
            TokenType::True,
            TokenType::RightParen,
            TokenType::Eof,
        ];
        assert_eq!(result.len(), expected.len());
        assert_eq!(tt!(result), expected);
    }
}
