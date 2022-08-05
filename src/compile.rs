use std::ops::DerefMut;

use crate::chunk::{ChunkBuilder, Patch};
use crate::function::{Closure, Function, FunctionBuilder, FunctionType};
use crate::intern_string::SymbolTable;
use crate::opcodes::OpCode;
use crate::tokens::{Token, TokenType};
use crate::value::Value;

const SUPER: [char; 5] = ['s', 'u', 'p', 'e', 'r'];

macro_rules! emit_opcodes {
        ($instance:ident, $($opcode:expr $(,)?),+ $(,)?) => {{
              $($instance.emit_opcode($opcode);)+
        }};
}

pub struct Parser<'a, I: Iterator<Item = Token<'a>>> {
    source: I,
    current: Token<'a>,
    previous: Token<'a>,
    had_error: bool,
    panic_mode: bool,
    rules: ParseRules<'a, I>,
    symbol_table: SymbolTable,
    compilers: Vec<Compiler<'a>>,
    class_compilers: Vec<ClassCompiler>,
}

impl<'a, I: Iterator<Item = Token<'a>>> Parser<'a, I> {
    pub fn new(source: I) -> Self {
        let mut parser = Parser {
            source,
            current: Token::new(TokenType::Error, &[], 0),
            previous: Token::new(TokenType::Error, &[], 0),
            had_error: false,
            panic_mode: false,
            rules: ParseRules::new(),
            symbol_table: SymbolTable::new(),
            compilers: Vec::new(),
            class_compilers: Vec::new(),
        };
        parser.compilers.push(Compiler::new(FunctionType::Script));
        parser.advance();
        parser
    }

    pub fn compile(mut self) -> Result<(Closure, SymbolTable), ()> {
        while !self.matches(TokenType::EOF) {
            self.declaration();
        }

        let function = self.end_compile();

        if self.had_error {
            Err(())
        } else {
            Ok((Closure::new(function), self.symbol_table))
        }
    }
}

impl<'a, I: Iterator<Item = Token<'a>>> Parser<'a, I> {
    fn declaration(&mut self) {
        if self.matches(TokenType::Class) {
            self.class_declaration();
        } else if self.matches(TokenType::Fun) {
            self.function_declaration();
        } else if self.matches(TokenType::Var) {
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
        } else if self.matches(TokenType::If) {
            self.if_statement();
        } else if self.matches(TokenType::While) {
            self.while_statement();
        } else if self.matches(TokenType::For) {
            self.for_statement();
        } else if self.matches(TokenType::Return) {
            self.return_statement();
        } else if self.matches(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
        } else {
            self.expression_statement();
        }
    }

    fn if_statement(&mut self) {
        self.consume(TokenType::LeftParen, "Expected '(' after 'if'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after condition.");

        let then_branch = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_opcode(OpCode::OpPop);
        self.statement();
        let else_branch = self.emit_jump(OpCode::OpJump);
        self.patch_jump(then_branch);
        self.emit_opcode(OpCode::OpPop);

        if self.matches(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_branch);
    }

    fn for_statement(&mut self) {
        // Variables decleared in a for-loop live in their own scope.
        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' after 'for'.");

        // Initializer clause is optional and can be an expression statement or a variable declaration.
        if self.matches(TokenType::Semicolon) {
            // No initialization.
        } else if self.matches(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().len();

        // The exit condition is optional.
        let exit_jump = if !self.matches(TokenType::Semicolon) {
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';' after loop condition.");
            let jmp = self.emit_jump(OpCode::OpJumpIfFalse);
            self.emit_opcode(OpCode::OpPop);
            Some(jmp)
        } else {
            None
        };

        // The increment clause is optional.
        if !self.matches(TokenType::RightParen) {
            let body_jump = self.emit_jump(OpCode::OpJump);
            let inc_start = self.current_chunk().len();
            self.expression();
            self.emit_opcode(OpCode::OpPop);
            self.consume(TokenType::RightParen, "Expected ')' after for clause.");

            self.emit_loop(loop_start);
            loop_start = inc_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(jump) = exit_jump {
            self.patch_jump(jump);
            self.emit_opcode(OpCode::OpPop);
        }

        self.end_scope();
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().len();
        self.consume(TokenType::LeftParen, "Expected '(' after 'while'.");
        self.expression();
        self.consume(TokenType::RightParen, "Expected ')' after condition.");

        let exit_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_opcode(OpCode::OpPop);
        self.statement();
        self.emit_loop(loop_start);
        self.patch_jump(exit_jump);
        self.emit_opcode(OpCode::OpPop);
    }

    fn patch_jump(&mut self, patch: Patch) {
        let distance = self.current_chunk().len() - patch.get_own_index() - 2;

        if distance > u16::MAX as usize {
            self.error("Too much code to jump over.");
            // Safety: There is an error and the resulting chunk will not be valid code and thus not
            //   be executed. So just writing 0 is fine.
            unsafe { patch.apply(0) };
        } else {
            // Safety: Distance points to the current position which always is an opcode when this
            // function is called.
            unsafe { patch.apply(distance as u16) }
        }
    }

    fn function_declaration(&mut self) {
        let global = self.parse_variable("Expected function name.");
        self.current_compiler().mark_local_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
    }

    fn function(&mut self, kind: FunctionType) {
        self.compilers.push(Compiler::new(kind));
        self.current_compiler()
            .get_function_builder()
            .set_kind(kind);
        if kind != FunctionType::Script {
            let name = self.previous.get_lexme_string();
            let intern = self.symbol_table.intern(name);
            self.current_compiler()
                .get_function_builder()
                .set_name(intern);
        }

        self.begin_scope();
        self.consume(TokenType::LeftParen, "Expected '(' after function name.");

        if !self.check(TokenType::RightParen) {
            loop {
                let function = self.current_compiler().get_function_builder();
                function.inc_arity(1);

                if function.get_arity() > 255 {
                    self.error_at_current("Can't have more than 255 parameter names.");
                }

                let constant = self.parse_variable("Expected parameter name.");
                self.define_variable(constant);

                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after parameters.");
        self.consume(TokenType::LeftBrace, "Expected '{' before function body.");

        self.block();

        let upvalues = self
            .current_compiler()
            .get_upvalues()
            .iter()
            .map(|v| (v.is_local() as u8, v.get_index()))
            .collect::<Vec<(u8, u8)>>();

        let function = self.end_compile();
        self.emit_opcode(OpCode::OpClosure);
        let index = self.make_constant(Value::Function(function));
        self.emit_index(index);

        upvalues.iter().for_each(|(l, i)| {
            self.emit_index(*l);
            self.emit_index(*i)
        });
    }

    fn class_declaration(&mut self) {
        self.consume(TokenType::Identifier, "Expected class name.");
        let class_name = self.previous.clone();
        let name = self.identifier_constant(self.previous.get_lexme_string());
        self.declare_variable();

        self.class_compilers.push(ClassCompiler::new());

        self.emit_opcode(OpCode::OpClass);
        self.emit_index(name);
        self.define_variable(name);

        if self.matches(TokenType::Less) {
            self.consume(TokenType::Identifier, "Expect superclass name.");
            self.variable(false);

            if class_name.get_lexme() == self.previous.get_lexme() {
                self.error("A class cannot inherit from itself.");
            }

            self.begin_scope();
            let dummy_token = self.synthetic_token(TokenType::Identifier, &SUPER);
            self.add_local(dummy_token);
            self.define_variable(0);

            self.named_variable(class_name.clone(), false);
            self.emit_opcode(OpCode::OpInherit);
            self.current_class_compiler_mut().set_has_superclass(true);
        }

        self.named_variable(class_name, false);
        self.consume(TokenType::LeftBrace, "Expected '{' before class body.");
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.method();
        }
        self.consume(TokenType::RightBrace, "Expected '}' after class body.");
        self.emit_opcode(OpCode::OpPop);

        if self.current_class_compiler().get_has_superclass() {
            self.end_scope();
        }

        self.class_compilers.pop();
    }

    fn method(&mut self) {
        self.consume(TokenType::Identifier, "Expected method name.");
        let constant = self.identifier_constant(self.previous.get_lexme_string());
        let kind = match self.previous.get_lexme_string() == "init" {
            true => FunctionType::Initializer,
            false => FunctionType::Method,
        };
        self.function(kind);
        self.emit_opcode(OpCode::OpMethod);
        self.emit_index(constant);
    }

    fn call(&mut self) {
        let arg_count = self.argument_list();
        self.emit_opcode(OpCode::OpCall);
        self.emit_index(arg_count);
    }

    fn dot(&mut self, can_assign: bool) {
        self.consume(TokenType::Identifier, "Expected property name after '.'.");
        let name = self.identifier_constant(self.previous.get_lexme_string());

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_opcode(OpCode::OpSetProperty);
            self.emit_index(name);
        } else if self.matches(TokenType::LeftParen) {
            let arg_count = self.argument_list();
            self.emit_opcode(OpCode::OpInvoke);
            self.emit_index(name);
            self.emit_index(arg_count);
        } else {
            self.emit_opcode(OpCode::OpGetProperty);
            self.emit_index(name);
        }
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count: u8 = 0;

        if !self.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.error("Can't have more than 255 arguments.");
                }
                arg_count += 1;

                if !self.matches(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after arguments.");
        arg_count
    }

    fn return_statement(&mut self) {
        if self.current_compiler().get_function_builder().get_kind() == FunctionType::Script {
            self.error("Can't return from top-level code.");
        }

        if self.matches(TokenType::Semicolon) {
            self.emit_return();
        } else {
            if self.current_compiler().get_function_builder().get_kind()
                == FunctionType::Initializer
            {
                self.error("Cannot return a value from an initializer.");
            }
            self.expression();
            self.consume(TokenType::Semicolon, "Expected ';' after return value.");
            self.emit_opcode(OpCode::OpReturn);
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

        self.declare_variable();
        if self.current_compiler().get_scope_depth() > 0 {
            0
        } else {
            self.identifier_constant(self.previous.get_lexme_string())
        }
    }

    fn identifier_constant(&mut self, name: String) -> u8 {
        let intern = self.symbol_table.intern(name);
        self.make_constant(Value::String(intern))
    }

    fn declare_variable(&mut self) {
        if self.current_compiler().get_scope_depth() > 0 {
            let name = self.previous.clone();
            if !self
                .current_compiler()
                .check_variable_declared_in_current_scope(&name)
            {
                self.add_local(name);
            } else {
                self.error("Already declared a variable with this name in this scope.");
            }
        }
    }

    fn define_variable(&mut self, global: u8) {
        if self.current_compiler().get_scope_depth() == 0 {
            self.emit_opcode(OpCode::OpDefineGlobal);
            self.emit_index(global);
        } else {
            self.current_compiler().mark_local_initialized();
        }
    }

    fn add_local(&mut self, name: Token<'a>) {
        if self.current_compiler().get_local_count() < (u8::MAX as usize) {
            let local = Local::new(name, -1);
            self.current_compiler().push_local(local);
        } else {
            self.error("Too many local variables in function.");
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.consume(TokenType::Semicolon, "Expected ';' after value.");
        self.emit_opcode(OpCode::OpPrint);
    }

    fn block(&mut self) {
        while !self.check(TokenType::RightBrace) && !self.check(TokenType::EOF) {
            self.declaration();
        }

        self.consume(TokenType::RightBrace, "Expected '}’ after block.");
    }

    fn begin_scope(&mut self) {
        self.current_compiler().inc_scope_depth();
    }

    fn end_scope(&mut self) {
        self.current_compiler().dec_scope_depth();
        let is_captured = self.current_compiler().remove_out_of_scope_locals();
        is_captured
            .iter()
            .map(|c| {
                if *c {
                    OpCode::OpCloseUpvalue
                } else {
                    OpCode::OpPop
                }
            })
            .for_each(|op| self.emit_opcode(op));
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
        self.named_variable(self.previous.clone(), can_assign);
    }

    fn synthetic_token(&mut self, token_type: TokenType, text: &'static [char]) -> Token<'static> {
        Token::new(token_type, text, u32::MAX)
    }

    fn this(&mut self) {
        if self.class_compilers.is_empty() {
            self.error("Cannot use 'this' outside of a class");
        } else {
            self.variable(false);
        }
    }

    fn named_variable(&mut self, name: Token<'a>, can_assign: bool) {
        let (mut arg, uninitialized) = self.current_compiler().resolve(&name);
        if uninitialized {
            self.error("Can't read local variable in its own initializer.");
        }

        let (get, set) = if arg != -1 {
            (OpCode::OpGetLocal, OpCode::OpSetLocal)
        } else {
            arg = self.resolve_upvalue(self.compilers.len() - 1, &name);
            if arg != -1 {
                (OpCode::OpGetUpvalue, OpCode::OpSetUpvalue)
            } else {
                arg = self.identifier_constant(name.get_lexme_string()) as isize;
                (OpCode::OpGetGlobal, OpCode::OpSetGlobal)
            }
        };

        if can_assign && self.matches(TokenType::Equal) {
            self.expression();
            self.emit_opcode(set);
        } else {
            self.emit_opcode(get);
        }

        self.emit_index(arg as u8);
    }

    fn resolve_upvalue(&mut self, depth: usize, token: &Token) -> isize {
        if depth >= 1 {
            let next = depth - 1;
            let c = &mut self.compilers[next];
            let (local, _) = c.resolve(token);
            if local != -1 {
                c.get_local_at_mut(local as usize).set_captured(true);
                self.add_upvalue(depth, local as u8, true)
            } else {
                let upvalue = self.resolve_upvalue(next, token);
                if upvalue != -1 {
                    self.add_upvalue(depth, upvalue as u8, false)
                } else {
                    -1
                }
            }
        } else {
            -1
        }
    }

    fn add_upvalue(&mut self, compiler_index: usize, index: u8, is_local: bool) -> isize {
        let upvalue = Upvalue::new(index, is_local);
        match self.compilers[compiler_index].add_upvalue(upvalue) {
            Some(i) => i as isize,
            None => {
                self.error("Too many closures variables in function.");
                0
            }
        }
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
        let lexme = self.previous.get_lexme();
        let string = lexme[1..lexme.len() - 1].iter().collect::<String>();
        let intern = self.symbol_table.intern(string);
        self.emit_constant(Value::String(intern));
    }

    fn and(&mut self) {
        let end_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        self.emit_opcode(OpCode::OpPop);
        self.parse_precedence(Precedence::And);
        self.patch_jump(end_jump);
    }

    fn or(&mut self) {
        let else_jump = self.emit_jump(OpCode::OpJumpIfFalse);
        let end_jump = self.emit_jump(OpCode::OpJump);
        self.patch_jump(else_jump);
        self.emit_opcode(OpCode::OpPop);
        self.parse_precedence(Precedence::Or);
        self.patch_jump(end_jump);
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
        match self.current_compiler().get_function_builder().get_kind() {
            FunctionType::Initializer => {
                self.emit_opcode(OpCode::OpGetLocal);
                self.emit_index(0)
            }
            _ => self.emit_opcode(OpCode::OpNil),
        }

        self.emit_opcode(OpCode::OpReturn);
    }

    fn emit_opcode(&mut self, opcode: OpCode) {
        let line = self.previous.get_line();
        self.current_chunk().write_opcode(opcode, line);
    }

    fn emit_index(&mut self, index: u8) {
        self.current_chunk().write_index(index);
    }

    fn emit_address(&mut self, position: u16) {
        self.current_chunk().write_address(position);
    }

    fn emit_jump(&mut self, opcode: OpCode) -> Patch {
        assert!(matches!(opcode, OpCode::OpJump | OpCode::OpJumpIfFalse));
        self.emit_opcode(opcode);
        self.current_chunk().write_patch()
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_opcode(OpCode::OpLoop);

        let offset = self.current_chunk().len() - loop_start + 2;

        if offset > u16::MAX as usize {
            self.error("Loop body too large.");
            self.emit_address(0);
        } else {
            self.emit_address(offset as u16);
        }
    }

    fn current_chunk(&mut self) -> &mut ChunkBuilder {
        self.current_compiler().get_function_builder().deref_mut()
    }

    fn current_compiler(&mut self) -> &mut Compiler<'a> {
        self.compilers.last_mut().unwrap()
    }

    fn current_class_compiler(&self) -> &ClassCompiler {
        self.class_compilers.last().unwrap()
    }

    fn current_class_compiler_mut(&mut self) -> &mut ClassCompiler {
        self.class_compilers.last_mut().unwrap()
    }

    fn end_compile(&mut self) -> Function {
        self.emit_return();

        if !self.had_error {
            #[cfg(debug_assertions)]
            {
                let name = self
                    .current_compiler()
                    .get_function_builder()
                    .get_name()
                    .map_or(String::from("<script>"), |s| String::clone(s));
                let _ = self.current_chunk().print_disassemble(name.as_str());
            }
        }
        self.compilers.pop().unwrap().compile()
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
                return;
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

type ParseFn<'a, I> = fn(compiler: &mut Parser<'a, I>, can_assign: bool);

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
    rules: enum_map::EnumMap<TokenType, ParseRule<'a, I>>,
}

impl<'a, I: Iterator<Item = Token<'a>>> ParseRules<'a, I> {
    fn new() -> Self {
        let rules = enum_map::enum_map! {
        TokenType::LeftParen    => ParseRule::new(Some(|c, _| c.grouping()), Some(|c, _| c.call()), Precedence::Call),
            TokenType::RightParen   => ParseRule::new(None, None, Precedence::None),
            TokenType::LeftBrace    => ParseRule::new(None, None, Precedence::None),
            TokenType::RightBrace   => ParseRule::new(None, None, Precedence::None),
            TokenType::Comma        => ParseRule::new(None, None, Precedence::None),
            TokenType::Dot          => ParseRule::new(None, Some(|c, can_assign| c.dot(can_assign)),Precedence::Call),
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
            TokenType::And          => ParseRule::new(None, Some(|c, _| c.and()), Precedence::And),
            TokenType::Class        => ParseRule::new(None, None, Precedence::None),
            TokenType::Else         => ParseRule::new(None, None, Precedence::None),
            TokenType::False        => ParseRule::new(Some(|c, _| c.literal()), None, Precedence::None),
            TokenType::Fun          => ParseRule::new(None, None, Precedence::None),
            TokenType::For          => ParseRule::new(None, None, Precedence::None),
            TokenType::If           => ParseRule::new(None, None, Precedence::None),
            TokenType::Nil          => ParseRule::new(Some(|c, _| c.literal()), None, Precedence::None),
            TokenType::Or           => ParseRule::new(None, Some(|c, _| c.or()), Precedence::Or),
            TokenType::Print        => ParseRule::new(None, None, Precedence::None),
            TokenType::Return       => ParseRule::new(None, None, Precedence::None),
            TokenType::Super        => ParseRule::new(None, None, Precedence::None),
            TokenType::This         => ParseRule::new(Some(|c, _| c.this()), None, Precedence::None),
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

struct Compiler<'a> {
    function_builder: FunctionBuilder,
    locals: Vec<Local<'a>>,
    upvalues: Vec<Upvalue>,
    scope_depth: usize,
}

impl<'a> Compiler<'a> {
    fn new(kind: FunctionType) -> Self {
        let token = if kind != FunctionType::Function {
            Token::new(TokenType::EOF, &['t', 'h', 'i', 's'], 0)
        } else {
            Token::new(TokenType::EOF, &[], 0)
        };

        // We reserve the fist locals entry for internal use.
        let local = Local::new(token, 0);

        Compiler {
            function_builder: FunctionBuilder::new(None, 0, kind),
            locals: vec![local],
            upvalues: Vec::new(),
            scope_depth: 0,
        }
    }

    fn inc_scope_depth(&mut self) {
        self.scope_depth += 1;
    }

    fn dec_scope_depth(&mut self) {
        self.scope_depth -= 1;
    }

    fn get_scope_depth(&self) -> usize {
        self.scope_depth
    }

    fn push_local(&mut self, local: Local<'a>) {
        self.locals.push(local);
    }

    fn get_local_count(&self) -> usize {
        self.locals.len()
    }

    fn get_local_at_mut(&mut self, index: usize) -> &mut Local<'a> {
        &mut self.locals[index]
    }

    fn mark_local_initialized(&mut self) {
        if self.scope_depth > 0 {
            self.locals
                .last_mut()
                .unwrap()
                .set_depth(self.scope_depth as isize);
        }
    }

    fn check_variable_declared_in_current_scope(&self, name: &Token<'a>) -> bool {
        self.locals
            .iter()
            .rev()
            .take_while(|l| l.get_depth() == -1 || l.get_depth() >= self.scope_depth as isize)
            .any(|l| name.get_lexme() == l.get_name().get_lexme())
    }

    fn remove_out_of_scope_locals(&mut self) -> Vec<bool> {
        let mut is_captured: Vec<bool> = Vec::new();

        while self
            .locals
            .last()
            .map_or(false, |l| l.get_depth() > self.scope_depth as isize)
        {
            let close_upvalue = self.locals.last().unwrap().is_captured();
            is_captured.push(close_upvalue);
            self.locals.pop();
        }

        is_captured
    }

    fn resolve(&self, name: &Token<'a>) -> (isize, bool) {
        self.locals
            .iter()
            .enumerate()
            .rev()
            .find(|(_, l)| l.get_name().get_lexme() == name.get_lexme())
            .map_or((-1, false), |(i, l)| (i as isize, l.get_depth() == -1))
    }

    fn get_function_builder(&mut self) -> &mut FunctionBuilder {
        &mut self.function_builder
    }

    fn get_upvalues(&self) -> &Vec<Upvalue> {
        &self.upvalues
    }

    fn add_upvalue(&mut self, upvalue: Upvalue) -> Option<usize> {
        let value = self.upvalues.iter().enumerate().find(|(_, u)| {
            u.get_index() == upvalue.get_index() && u.is_local() == upvalue.is_local()
        });

        if let Some((i, _)) = value {
            Some(i)
        } else if self.upvalues.len() == u8::MAX as usize {
            None
        } else {
            self.upvalues.push(upvalue);
            self.get_function_builder().inc_upvalue_count();
            Some(self.upvalues.len() - 1)
        }
    }

    fn compile(self) -> Function {
        self.function_builder.build()
    }
}

struct Local<'a> {
    name: Token<'a>,
    depth: isize,
    is_captured: bool,
}

impl<'a> Local<'a> {
    fn new(name: Token<'a>, depth: isize) -> Self {
        Local {
            name,
            depth,
            is_captured: false,
        }
    }

    fn get_name(&self) -> &Token<'a> {
        &self.name
    }

    fn get_depth(&self) -> isize {
        self.depth
    }

    fn set_depth(&mut self, depth: isize) {
        self.depth = depth;
    }

    fn set_captured(&mut self, is_captured: bool) {
        self.is_captured = is_captured;
    }

    fn is_captured(&self) -> bool {
        self.is_captured
    }
}

pub struct Upvalue {
    index: u8,
    is_local: bool,
}

impl Upvalue {
    pub fn new(index: u8, is_local: bool) -> Self {
        Upvalue { index, is_local }
    }

    pub fn get_index(&self) -> u8 {
        self.index
    }

    pub fn is_local(&self) -> bool {
        self.is_local
    }
}

struct ClassCompiler {
    has_superclass: bool,
}

impl ClassCompiler {
    fn new() -> Self {
        ClassCompiler {
            has_superclass: false,
        }
    }

    fn get_has_superclass(&self) -> bool {
        self.has_superclass
    }

    fn set_has_superclass(&mut self, has_superclass: bool) {
        self.has_superclass = has_superclass;
    }
}
