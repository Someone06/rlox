use crate::chunk::{Chunk, ChunkBuilder, OpCode, Value};
use crate::intern_string::SymbolTable;
use crate::tokens::{Token, TokenType};

macro_rules! emit_opcodes {
        ($instance:ident, $($opcode:expr $(,)?),+ $(,)?) => {{
              $($instance.emit_opcode($opcode);)+
        }};
}

pub struct Compiler<'a, I: Iterator<Item = Token<'a>>> {
    source: I,
    current: Token<'a>,
    previous: Token<'a>,
    chunk: ChunkBuilder,
    had_error: bool,
    panic_mode: bool,
    rules: ParseRules<'a, I>,
    symbol_table: SymbolTable,
}

impl<'a, I: Iterator<Item = Token<'a>>> Compiler<'a, I> {
    pub fn new(source: I) -> Self {
        let mut compiler = Compiler {
            source,
            current: Token::new(TokenType::Error, &[], 0),
            previous: Token::new(TokenType::Error, &[], 0),
            chunk: ChunkBuilder::new(),
            had_error: false,
            panic_mode: false,
            rules: ParseRules::new(),
            symbol_table: SymbolTable::new(),
        };

        compiler.advance();
        compiler
    }

    pub fn compile(mut self) -> Result<(Chunk, SymbolTable), ()> {
        while !self.matches(TokenType::EOF) {
            self.declaration();
        }

        self.end_compile();

        #[cfg(debug_assertions)]
        let _ = self.current_chunk().print_disassemble("code");

        if self.had_error {
            Err(())
        } else {
            Ok((self.chunk.build(), self.symbol_table))
        }
    }
}

impl<'a, I: Iterator<Item = Token<'a>>> Compiler<'a, I> {
    fn declaration(&mut self) {
        if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.panic_mode {
            self.synchronize();
        }
    }

    fn synchronize(&mut self) {
        self.panic_mode = false;

        while !self.check(TokenType::EOF) {
            if self.previous.get_token_type() == TokenType::Semicolon {
                return;
            }

            if matches!(
                self.current.get_token_type(),
                TokenType::Class
                    | TokenType::Fun
                    | TokenType::Var
                    | TokenType::For
                    | TokenType::If
                    | TokenType::While
                    | TokenType::Print
                    | TokenType::Return
            ) {
                return;
            }

            self.advance();
        }
    }

    fn statement(&mut self) {
        if self.matches(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expected variable name.");
        if self.matches(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_opcode(OpCode::OpNil);
        }

        self.consume(
            TokenType::Semicolon,
            "Expected ';' after variable declaration.",
        );
        self.define_variable(global);
    }

    fn parse_variable(&mut self, error_message: &str) -> u8 {
        self.consume(TokenType::Identifier, error_message);
        self.identifier_constant(self.previous.get_lexme_string())
    }

    fn identifier_constant(&mut self, name: String) -> u8 {
        let intern = self.symbol_table.intern(name);
        self.make_constant(Value::String(intern))
    }

    fn define_variable(&mut self, global: u8) {
        self.emit_opcode(OpCode::OpDefineGlobal);
        self.emit_index(global);
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_opcode(OpCode::OpPrint);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after expression.");
        self.emit_opcode(OpCode::OpPop);
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn binary(&mut self) {
        let operator = self.previous.get_token_type();
        let parse_rule = self.rules.get(operator);
        let precedence = parse_rule.get_precedence().one_higher();
        self.parse_precedence(precedence);

        match &operator {
            TokenType::BangEqual => emit_opcodes!(self, OpCode::OpEqual, OpCode::OpNot),
            TokenType::EqualEqual => self.emit_opcode(OpCode::OpEqual),
            TokenType::Greater => self.emit_opcode(OpCode::OpGreater),
            TokenType::GreaterEqual => emit_opcodes!(self, OpCode::OpLess, OpCode::OpNot),
            TokenType::Less => self.emit_opcode(OpCode::OpLess),
            TokenType::LessEqual => emit_opcodes!(self, OpCode::OpGreater, OpCode::OpNot),
            TokenType::Plus => self.emit_opcode(OpCode::OpAdd),
            TokenType::Minus => self.emit_opcode(OpCode::OpSubtract),
            TokenType::Star => self.emit_opcode(OpCode::OpMultiply),
            TokenType::Slash => self.emit_opcode(OpCode::OpDivide),
            _ => unreachable!(),
        }
    }

    fn unary(&mut self) {
        let operator_type = self.previous.get_token_type();
        self.parse_precedence(Precedence::Unary);
        match operator_type {
            TokenType::Bang => self.emit_opcode(OpCode::OpNot),
            TokenType::Minus => self.emit_opcode(OpCode::OpNegate),
            _ => unreachable!(),
        }
    }

    fn grouping(&mut self) {
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after expression.");
    }

    fn variable(&mut self, can_assign: bool) {
        self.named_variable(self.previous.get_lexme_string(), can_assign);
    }

    fn named_variable(&mut self, name: String, can_assign: bool) {
        let index = self.identifier_constant(name);
        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_opcode(OpCode::OpSetGlobal);
        } else {
            self.emit_opcode(OpCode::OpGetGlobal);
        }

        self.emit_index(index);
    }

    fn number(&mut self) {
        let value = self
            .previous
            .get_lexme_string()
            .parse::<f64>()
            .expect("Expected the lexme to be a number.");
        self.emit_constant(Value::Double(value));
    }

    fn literal(&mut self) {
        match self.previous.get_token_type() {
            TokenType::True => self.emit_opcode(OpCode::OpTrue),
            TokenType::False => self.emit_opcode(OpCode::OpFalse),
            TokenType::Nil => self.emit_opcode(OpCode::OpNil),
            _ => unreachable!(),
        }
    }

    fn string(&mut self) {
        let intern = self.symbol_table.intern(self.previous.get_lexme_string());
        self.emit_constant(Value::String(intern));
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let tt = self.previous.get_token_type();
        let parse_rule = self.rules.get(tt);

        let can_assign = precedence <= Precedence::Assignment;
        if let Some(ref prefix_rule) = parse_rule.get_prefix() {
            prefix_rule(self, can_assign);
        } else {
            self.error("Expected expression.");
        }

        while precedence
            <= self
                .rules
                .get(self.current.get_token_type())
                .get_precedence()
        {
            self.advance();
            let infix_rule = self
                .rules
                .get(self.previous.get_token_type())
                .get_infix()
                .unwrap();
            infix_rule(self, can_assign);
        }

        if can_assign && self.matches(TokenType::Equal) {
            self.error("Invalid assignment target.");
        }
    }

    fn emit_constant(&mut self, value: Value) {
        self.emit_opcode(OpCode::OpConstant);
        let index = self.make_constant(value);
        self.emit_index(index);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let index = self.current_chunk().add_constant(value);
        if index > u8::MAX as usize {
            self.error("Too many constants in one chunk.");
            0
        } else {
            index as u8
        }
    }

    fn emit_return(&mut self) {
        self.emit_opcode(OpCode::OpReturn);
    }

    fn end_compile(&mut self) {
        self.emit_return();
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        let line = self.previous.get_line();
        self.current_chunk().write_opcode(opcode, line);
    }

    fn emit_index(&mut self, index: u8) {
        self.current_chunk().write_index(index);
    }

    fn current_chunk(&mut self) -> &mut ChunkBuilder {
        &mut self.chunk
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if !self.matches(token_type) {
            self.error_at_current(message);
        }
    }

    fn matches(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.current.get_token_type() == token_type
    }

    fn advance(&mut self) {
        let mut current: Option<Token<'a>>;

        loop {
            current = self.source.next();

            if let Some(token) = current {
                match &token.get_token_type() {
                    TokenType::Error => {
                        self.error_at(&token, &token.get_lexme_string());
                    }
                    _ => {
                        self.previous = std::mem::replace(&mut self.current, token);
                        return;
                    }
                }
            } else {
                panic!("Exhausted token stream without hitting EOF token.");
            }
        }
    }

    fn error(&mut self, message: &str) {
        if !self.panic_mode {
            self.panic_mode = true;
            self.had_error = true;
            error_at(&self.previous, message);
        }
    }

    fn error_at_current(&mut self, message: &str) {
        if !self.panic_mode {
            self.panic_mode = true;
            self.had_error = true;
            error_at(&self.current, message);
        }
    }

    fn error_at(&mut self, token: &Token<'a>, message: &str) {
        if !self.panic_mode {
            self.panic_mode = true;
            self.had_error = true;
            error_at(token, message);
        }
    }
}

fn error_at<'a>(token: &Token<'a>, message: &str) {
    eprint!("[line {}] Error", token.get_line());

    if token.get_token_type() == TokenType::EOF {
        eprint!(" at end");
    } else if token.get_token_type() != TokenType::Error {
        eprint!(" at '{}'", token.get_lexme_string())
    }

    eprintln!(": {}", message);
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    fn one_higher(&self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => panic!("Primary is highest precedence!"),
        }
    }
}

impl std::fmt::Display for Precedence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

type ParseFn<'a, I> = fn(compiler: &mut Compiler<'a, I>, can_assign: bool);

struct ParseRule<'a, I: Iterator<Item = Token<'a>>> {
    prefix: Option<ParseFn<'a, I>>,
    infix: Option<ParseFn<'a, I>>,
    precedence: Precedence,
}

impl<'a, I: Iterator<Item = Token<'a>>> ParseRule<'a, I> {
    fn new(
        prefix: Option<ParseFn<'a, I>>,
        infix: Option<ParseFn<'a, I>>,
        precedence: Precedence,
    ) -> Self {
        ParseRule {
            prefix,
            infix,
            precedence,
        }
    }

    fn get_prefix(&self) -> Option<ParseFn<'a, I>> {
        self.prefix
    }

    fn get_infix(&self) -> Option<ParseFn<'a, I>> {
        self.infix
    }

    fn get_precedence(&self) -> Precedence {
        self.precedence
    }
}

struct ParseRules<'a, I: Iterator<Item = Token<'a>>> {
    rules: ::enum_map::EnumMap<TokenType, ParseRule<'a, I>>,
}

impl<'a, I: Iterator<Item = Token<'a>>> ParseRules<'a, I> {
    fn new() -> Self {
        let rules = ::enum_map::enum_map! {
        TokenType::LeftParen    => ParseRule::new(Some(|c, _| {c.grouping()}), None, Precedence::None),
            TokenType::RightParen   => ParseRule::new(None, None, Precedence::None),
            TokenType::LeftBrace    => ParseRule::new(None, None, Precedence::None),
            TokenType::RightBrace   => ParseRule::new(None, None, Precedence::None),
            TokenType::Comma        => ParseRule::new(None, None, Precedence::None),
            TokenType::Dot          => ParseRule::new(None, None, Precedence::None),
            TokenType::Minus        => ParseRule::new(Some(|c, _| c.unary()), Some(|c, _| c.binary()), Precedence::Term),
            TokenType::Plus         => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Term),
            TokenType::Semicolon    => ParseRule::new(None, None, Precedence::None),
            TokenType::Slash        => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Factor),
            TokenType::Star         => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Factor),
            TokenType::Bang         => ParseRule::new(Some(|c, _| c.unary()), None, Precedence::None),
            TokenType::BangEqual    => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Equality),
            TokenType::Equal        => ParseRule::new(None, None, Precedence::None),
            TokenType::EqualEqual   => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Equality),
            TokenType::Greater      => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Comparison),
            TokenType::GreaterEqual => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Comparison),
            TokenType::Less         => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Comparison),
            TokenType::LessEqual    => ParseRule::new(None, Some(|c, _| c.binary()), Precedence::Comparison),
            TokenType::Identifier   => ParseRule::new(Some(|c, can_assign | c.variable(can_assign)), None, Precedence::None),
            TokenType::String       => ParseRule::new(Some(|c, _| c.string()), None, Precedence::None),
            TokenType::Number       => ParseRule::new(Some(|c, _| {c.number()}), None, Precedence::None),
            TokenType::And          => ParseRule::new(None, None, Precedence::None),
            TokenType::Class        => ParseRule::new(None, None, Precedence::None),
            TokenType::Else         => ParseRule::new(None, None, Precedence::None),
            TokenType::False        => ParseRule::new(Some(|c, _| c.literal()), None, Precedence::None),
            TokenType::Fun          => ParseRule::new(None, None, Precedence::None),
            TokenType::For          => ParseRule::new(None, None, Precedence::None),
            TokenType::If           => ParseRule::new(None, None, Precedence::None),
            TokenType::Nil          => ParseRule::new(Some(|c, _| c.literal()), None, Precedence::None),
            TokenType::Or           => ParseRule::new(None, None, Precedence::None),
            TokenType::Print        => ParseRule::new(None, None, Precedence::None),
            TokenType::Return       => ParseRule::new(None, None, Precedence::None),
            TokenType::Super        => ParseRule::new(None, None, Precedence::None),
            TokenType::This         => ParseRule::new(None, None, Precedence::None),
            TokenType::True         => ParseRule::new(Some(|c, _| c.literal()), None, Precedence::None),
            TokenType::Var          => ParseRule::new(None, None, Precedence::None),
            TokenType::While        => ParseRule::new(None, None, Precedence::None),
            TokenType::Error        => ParseRule::new(None, None, Precedence::None),
            TokenType::EOF          => ParseRule::new(None, None, Precedence::None),
        };

        ParseRules { rules }
    }

    fn get(&self, token_type: TokenType) -> &ParseRule<'a, I> {
        &self.rules[token_type]
    }
}
