use std::collections::HashMap;
use std::io::Write;

use crate::chunk::{OpCode, Value};
use crate::function::{clock, Function, NativeFunction};
use crate::intern_string::{Symbol, SymbolTable};

#[derive(PartialEq, Eq, Debug)]
pub enum InterpretResult {
    CompileError,
    RuntimeError,
}

pub struct VM<O: Write> {
    frames: Vec<CallFrame>,
    stack: Vec<Value>,
    symbol_table: SymbolTable,
    globals: HashMap<Symbol, Value>,
    print_output: O,
}

impl VM<std::io::Stdout> {
    pub fn new(function: Function, symbol_table: SymbolTable) -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            symbol_table,
            globals: HashMap::new(),
            frames: Vec::new(),
            print_output: std::io::stdout(),
        };

        vm.stack.push(Value::Function(function.clone()));
        vm.call(function, 0);
        vm.define_native(String::from("clock"), NativeFunction::new(clock, 0));
        vm
    }
}

impl<O: Write> VM<O> {
    pub fn with_write(function: Function, symbol_table: SymbolTable, write: O) -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            symbol_table,
            globals: HashMap::new(),
            frames: Vec::new(),
            print_output: write,
        };

        vm.stack.push(Value::Function(function.clone()));
        vm.call(function, 0);
        vm.define_native(String::from("clock"), NativeFunction::new(clock, 0));
        vm
    }
}

impl<O: Write> VM<O> {
    pub fn interpret(mut self) -> Result<O, InterpretResult> {
        self.run().map(|_| self.print_output)
    }

    fn run(&mut self) -> Result<(), InterpretResult> {
        loop {
            // Safety: Initially, self.ip is zero, so it points to an opcode in self.chunk.
            //         Each time we execute the loop we ensure that self.ip again points to an opcode.
            let opcode = unsafe { self.read_opcode() };

            #[cfg(debug_assertions)]
            self.print_stack();

            // Safety: The last instruction read is an opcode and self.ip got incremented by one
            //         after reading it. So self.ip - 1 points to that opcode.
            #[cfg(debug_assertions)]
            unsafe {
                let frame = self.frames.last().unwrap();
                let chunk = frame.get_function().get_chunk();
                let ip = frame.get_ip();
                let _ = chunk.print_disassemble_instruction_unsafe(ip - 1);
            }

            match opcode {
                OpCode::OpReturn => {
                    let value = self.stack.pop().unwrap();
                    let frame = self.frames.pop().unwrap();

                    if self.frames.is_empty() {
                        // Reached end of program.
                        self.stack.pop();
                        return Ok(());
                    } else {
                        self.stack.truncate(frame.get_slots());
                        self.stack.push(value);
                    }
                }
                OpCode::OpPrint => {
                    let _ = write!(self.print_output, "{}\n", self.stack.pop().unwrap());
                }
                OpCode::OpPop => {
                    self.stack.pop();
                }
                OpCode::OpDefineGlobal => {
                    // Safety: OpDefineGlobal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let name = unsafe { self.read_constant() }.clone();
                    if let Value::String(n) = name {
                        let value = self.stack.pop().unwrap().clone();
                        self.globals.insert(n, value);
                    } else {
                        unreachable!("OpDefineGlobal has an index pointing to a string which is enforced int the compiler.");
                    }
                }
                OpCode::OpGetGlobal => {
                    // Safety: OpGetGlobal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let name = unsafe { self.read_constant() }.clone();
                    if let Value::String(ref n) = name {
                        let value = self.globals.get(n);
                        match value {
                            Some(v) => self.stack.push(v.clone()),
                            None => {
                                self.runtime_error(format!("Undefined variable '{}'.", n).as_str());
                                return Err(InterpretResult::RuntimeError);
                            }
                        }
                    } else {
                        unreachable!("OpGetGlobal has an index pointing to a string which is enforced int the compiler.");
                    }
                }
                OpCode::OpSetGlobal => {
                    // Safety: OpSetGlobal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let name = unsafe { self.read_constant() }.clone();
                    if let Value::String(ref n) = name {
                        let value = self.globals.get_mut(n);
                        match value {
                            Some(v) => *v = self.stack.last().unwrap().clone(),
                            None => {
                                self.runtime_error(format!("Undefined variable '{}'.", n).as_str());
                                return Err(InterpretResult::RuntimeError);
                            }
                        }
                    } else {
                        unreachable!("OpSetGlobal has an index pointing to a string which is enforced int the compiler.");
                    }
                }
                OpCode::OpGetLocal => {
                    // Safety: OpGetLocal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let slot = unsafe { self.read_index() };
                    let value = self.stack[slot as usize].clone();
                    self.stack.push(value);
                }
                OpCode::OpSetLocal => {
                    // Safety: OpSetLocal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let slot = unsafe { self.read_index() };
                    let value = self.stack.last().unwrap().clone();
                    self.stack[slot as usize] = value;
                }
                OpCode::OpNegate => {
                    match self
                        .stack
                        .last_mut()
                        .expect("Stack should not be empty when execution OpNegate.")
                    {
                        Value::Double(ref mut f) => *f *= -1.0,
                        _ => {
                            self.runtime_error("Operand must be a number.");
                            return Err(InterpretResult::RuntimeError);
                        }
                    }
                }
                OpCode::OpAdd => {
                    let b = self
                        .stack
                        .pop()
                        .expect("Expecting stack size at least 2 for binary op.");
                    let a = self
                        .stack
                        .pop()
                        .expect("Expecting stack size at least 2 for binary op.");

                    if let (Value::Double(f1), Value::Double(f2)) = (a.clone(), b.clone()) {
                        self.stack.push(Value::Double(f1 + f2));
                    } else if let (Value::String(s1), Value::String(s2)) = (a, b) {
                        let concat = format!("{}{}", s1, s2);
                        let intern = self.symbol_table.intern(concat);
                        self.stack.push(Value::String(intern));
                    } else {
                        self.runtime_error("Operands must be two numbers or two strings.");
                        return Err(InterpretResult::RuntimeError);
                    }
                }
                OpCode::OpSubtract => {
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Double(f1 - f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
                }
                OpCode::OpMultiply => {
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Double(f1 * f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
                }
                OpCode::OpDivide => {
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Double(f1 / f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
                }
                OpCode::OpNot => {
                    let value = Value::Bool(self.stack.pop().unwrap().is_falsey());
                    self.stack.push(value);
                }
                OpCode::OpEqual => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::Bool(a == b));
                }
                OpCode::OpLess => {
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Bool(f1 < f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
                }
                OpCode::OpGreater => {
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Bool(f1 > f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
                }

                OpCode::OpConstant => {
                    // Safety: We know that OpConstant takes one arguments to which self.ip points,
                    //         because it is incremented after reading this opcode.
                    //         Also self.ip gets incremented after reading the constant so it will
                    //         point to the next opcode after this.
                    let value = unsafe { self.read_constant() }.clone();
                    self.stack.push(value);
                }

                OpCode::OpTrue => self.stack.push(Value::Bool(true)),
                OpCode::OpFalse => self.stack.push(Value::Bool(false)),
                OpCode::OpNil => self.stack.push(Value::Nil),

                OpCode::OpJump => {
                    // Safety: We know that OpJump takes two arguments to which self.ip points, and
                    //         it is incremented by two after reading this opcode. The offset has
                    //         been calculated in the compiler s.t. self.ip points to an opcode
                    //         after increasing it by offset.
                    let offset = unsafe { self.read_short() };
                    self.frames.last_mut().unwrap().inc_ip(offset as usize);
                }
                OpCode::OpJumpIfFalse => {
                    // Safety: We know that OpJumpIfFalse takes two arguments to which self.ip
                    //         points, and it is incremented by two after reading this opcode.
                    //         If the current value is true-thy ip just points to the next opcode.
                    //         Else the offset has been calculated in the compiler s.t. self.ip
                    //         points to an opcode after increasing it by offset.
                    let offset = unsafe { self.read_short() };
                    if self.stack.last().unwrap().is_falsey() {
                        self.frames.last_mut().unwrap().inc_ip(offset as usize);
                    }
                }
                OpCode::OpLoop => {
                    // Safety: We know that OpLoop takes two arguments to which self.ip
                    //         points, and it is incremented by two after reading this opcode.
                    //         The offset has been calculated in the compiler s.t. self.ip
                    //         points to an opcode after decrementing it by offset.
                    let offset = unsafe { self.read_short() };
                    self.frames.last_mut().unwrap().dec_ip(offset as usize);
                }
                OpCode::OpCall => {
                    let arg_count = unsafe { self.read_index() };
                    let callee = self.stack[self.stack.len() - 1 - arg_count as usize].clone();
                    if !self.call_value(callee, arg_count) {
                        return Err(InterpretResult::RuntimeError);
                    }
                    self.frames.pop();
                }
            }
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: u8) -> bool {
        match callee {
            Value::Function(fun) => self.call(fun, arg_count),
            Value::NativeFunction(fun) => {
                if arg_count as usize == fun.get_arity() {
                    let args = &self.stack[self.stack.len() - arg_count as usize..];
                    let result = fun.call(args);
                    self.stack
                        .truncate(self.stack.len().saturating_sub(arg_count as usize + 1));
                    self.stack.push(result);
                    true
                } else {
                    self.runtime_error(
                        format!(
                            "Expected {} arguments, but got {}.",
                            fun.get_arity(),
                            arg_count
                        )
                        .as_str(),
                    );
                    false
                }
            }
            _ => {
                self.runtime_error("Can only call functions and classes.");
                false
            }
        }
    }

    fn call(&mut self, function: Function, arg_count: u8) -> bool {
        if arg_count as usize == function.get_arity() {
            let frame = CallFrame::new(function, 0, self.stack.len() - arg_count as usize - 1);
            self.frames.push(frame);
            true
        } else {
            self.runtime_error(
                format!(
                    "Expected {} arguments, but got {}.",
                    function.get_arity(),
                    arg_count
                )
                .as_str(),
            );
            false
        }
    }

    fn define_native(&mut self, name: String, function: NativeFunction) {
        let intern = self.symbol_table.intern(name);
        self.globals.insert(intern, Value::NativeFunction(function));
    }

    fn binary_double_op(
        &mut self,
        op: impl Fn(Value, Value) -> Result<Value, InterpretResult>,
    ) -> Result<(), InterpretResult> {
        let b = self
            .stack
            .pop()
            .expect("Expecting stack size at least 2 for binary op.");
        let a = self
            .stack
            .pop()
            .expect("Expecting stack size at least 2 for binary op.");
        match op(a, b) {
            Ok(result) => {
                self.stack.push(result);
                Ok(())
            }
            Err(error) => {
                self.runtime_error("Operands must be numbers.");
                Err(error)
            }
        }
    }

    fn reset_stack(&mut self) {
        self.stack.clear();
        self.frames.clear();
    }

    /// Safety: It is only safe to call this function when self.ip is the index of an index in
    /// self.chunk.
    unsafe fn read_index(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let chunk = frame.get_function().get_chunk();
        let ip = frame.get_ip();
        let code_unit = chunk.get_code_unit(ip);
        frame.inc_ip(1);
        code_unit.get_index()
    }

    /// Safety: It is only safe to call this function when self.ip is the index of an short value
    /// consisting of two consecutive indexes in self.chunk.
    unsafe fn read_short(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let chunk = frame.get_function().get_chunk();
        let ip = frame.get_ip();
        let code_unit_high = chunk.get_code_unit(ip);
        let code_unit_low = chunk.get_code_unit(ip + 1);
        frame.inc_ip(2);

        let high = code_unit_high.get_index();
        let low = code_unit_low.get_index();
        ((high as u16) << 8) + (low as u16)
    }

    /// Safety: It is only safe to call this function when self.ip is the index of an index in
    /// self.chunk.
    unsafe fn read_constant(&mut self) -> &Value {
        let index = self.read_index();
        let frame = self.frames.last().unwrap();
        let chunk = frame.get_function().get_chunk();
        chunk.get_value_at_index(index)
    }

    /// Safety: It's only save to call this function when self.ip is the index of an opcode in
    ///         self.chunk.
    unsafe fn read_opcode(&mut self) -> OpCode {
        let frame = self.frames.last_mut().unwrap();
        let chunk = frame.get_function().get_chunk();
        let ip = frame.get_ip();
        let code_unit = chunk.get_code_unit(ip);
        frame.inc_ip(1);
        code_unit.get_opcode()
    }

    fn runtime_error(&mut self, message: &str) {
        for frame in self.frames.iter().rev() {
            let function = frame.get_function();
            let ip = frame.get_ip() - 1;
            let name = match function.get_name() {
                Some(name) => name.as_str(),
                None => "script",
            };
            eprint!(
                "[line {}] in {}()",
                function.get_chunk().get_source_code_line(ip),
                name
            );
        }

        self.reset_stack();
    }

    #[cfg(debug_assertions)]
    fn print_stack(&self) {
        self.stack.iter().for_each(|value| print!("[{}]", value));
        println!();
    }
}

struct CallFrame {
    function: Function,
    ip: usize,
    slots: usize,
}

impl CallFrame {
    pub fn new(function: Function, ip: usize, slots: usize) -> Self {
        Self {
            function,
            ip,
            slots,
        }
    }

    pub fn get_function(&self) -> &Function {
        &self.function
    }
    pub fn get_ip(&self) -> usize {
        self.ip
    }

    pub fn set_ip(&mut self, position: usize) {
        self.ip = position;
    }

    pub fn inc_ip(&mut self, difference: usize) {
        self.ip += difference;
    }

    pub fn dec_ip(&mut self, difference: usize) {
        self.ip = (self.ip as isize - difference as isize) as usize;
    }

    pub fn get_slots(&self) -> usize {
        self.slots
    }
}

#[cfg(test)]
mod tests {
    use std::ops::DerefMut;

    use crate::chunk::{ChunkBuilder, OpCode, Value};
    use crate::function::{FunctionBuilder, FunctionType};
    use crate::intern_string::SymbolTable;
    use crate::vm::VM;

    macro_rules! load_constant {
        ($builder:ident, $index: literal, $value:literal, $line: literal) => {{
            $builder.write_opcode(OpCode::OpConstant, $line);
            $builder.write_index($index);
            $builder.add_constant(Value::Double($value));
        }};
    }

    macro_rules! check_result {
        ($builder:ident, $result: literal) => {{
            let mut builder = $builder;
            let mut function = FunctionBuilder::new(None, 0, FunctionType::Script);
            let function_chunk = function.deref_mut();
            std::mem::swap(function_chunk, &mut builder);
            let mut vm = VM::new(function.build(), SymbolTable::new());
            let result = vm.interpret().unwrap();
            match result {
                Value::Double(float) => assert_eq!($result, float.round() as isize),
                _ => panic!("Expected a double value."),
            }
        }};
    }

    #[test]
    fn test_calculate1() {
        // Computes 1 * 2 + 3 = 5.
        let mut builder = ChunkBuilder::new();
        load_constant!(builder, 0, 1.0, 0);
        load_constant!(builder, 1, 2.0, 0);
        builder.write_opcode(OpCode::OpMultiply, 1);
        load_constant!(builder, 2, 3.0, 2);
        builder.write_opcode(OpCode::OpAdd, 3);
        builder.write_opcode(OpCode::OpReturn, 5);

        check_result!(builder, 5);
    }

    #[test]
    fn test_calculate2() {
        // Computes 1 + 2 * 3 - 8 / -4 = 1 + 6 - (-2) = 9.
        let mut builder = ChunkBuilder::new();
        load_constant!(builder, 0, 1.0, 0);
        load_constant!(builder, 1, 2.0, 0);
        load_constant!(builder, 2, 3.0, 0);
        builder.write_opcode(OpCode::OpMultiply, 1);
        builder.write_opcode(OpCode::OpAdd, 2);
        load_constant!(builder, 3, 8.0, 2);
        load_constant!(builder, 4, 4.0, 2);
        builder.write_opcode(OpCode::OpNegate, 3);
        builder.write_opcode(OpCode::OpDivide, 3);
        builder.write_opcode(OpCode::OpSubtract, 3);
        builder.write_opcode(OpCode::OpReturn, 4);

        check_result!(builder, 9);
    }
}
