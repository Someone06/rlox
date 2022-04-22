use std::collections::HashMap;
use std::io::Write;
use std::ops::Deref;

use crate::function::{clock, Closure, NativeFunction, ObjUpvalue, UpvalueLocation};
use crate::intern_string::{Symbol, SymbolTable};
use crate::opcodes::OpCode;
use crate::value::Value;

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
    open_upvalues: Vec<ObjUpvalue>,
    print_output: O,
}

impl VM<std::io::Stdout> {
    pub fn new(closure: Closure, symbol_table: SymbolTable) -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            symbol_table,
            globals: HashMap::new(),
            frames: Vec::new(),
            open_upvalues: Vec::new(),
            print_output: std::io::stdout(),
        };

        vm.stack.push(Value::Closure(closure.clone()));
        vm.call(closure, 0);
        vm.define_native(String::from("clock"), NativeFunction::new(clock, 0));
        vm
    }
}

impl<O: Write> VM<O> {
    pub fn with_write(closure: Closure, symbol_table: SymbolTable, write: O) -> Self {
        let mut vm = VM {
            stack: Vec::new(),
            symbol_table,
            globals: HashMap::new(),
            frames: Vec::new(),
            open_upvalues: Vec::new(),
            print_output: write,
        };

        vm.stack.push(Value::Closure(closure.clone()));
        vm.call(closure, 0);
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
                let chunk = frame.get_closure().get_function().get_chunk();
                let ip = frame.get_ip();
                let _ = chunk.print_disassemble_instruction_unsafe(ip - 1);
            }

            match opcode {
                OpCode::OpReturn => {
                    let value = self.stack.pop().unwrap();
                    let frame = self.frames.pop().unwrap();
                    self.close_upvalues(frame.get_slots());

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
                    let frame = self.frames.last().unwrap();
                    let value = self.stack[frame.get_slots() + slot as usize].clone();
                    self.stack.push(value);
                }
                OpCode::OpSetLocal => {
                    // Safety: OpSetLocal requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let slot = unsafe { self.read_index() };
                    let frame = self.frames.last().unwrap();
                    let value = self.stack.last().unwrap().clone();
                    self.stack[frame.get_slots() + slot as usize] = value;
                }
                OpCode::OpGetUpvalue => {
                    // Safety: OpGetUpvalue requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let slot = unsafe { self.read_index() } as usize;
                    let frame = self.frames.last().unwrap();
                    let location = frame.get_closure().get_upvalue_at(slot).get_location();
                    let value = match location {
                        UpvalueLocation::Stack(offset) => self.stack[offset].clone(),
                        UpvalueLocation::Heap(rc) => rc.deref().clone(),
                    };
                    self.stack.push(value);
                }
                OpCode::OpSetUpvalue => {
                    // Safety: OpGetUpvalue requires a index. The index is written by the compiler
                    //         into the chunk and the chunk ensures that it is written.
                    let slot = unsafe { self.read_index() } as usize;
                    let value = self.stack.last().unwrap().clone();
                    let frame = self.frames.last_mut().unwrap();
                    if let UpvalueLocation::Stack(offset) =
                        frame.get_closure().get_upvalue_at(slot).get_location()
                    {
                        self.stack[offset] = value;
                    } else {
                        frame
                            .get_closure_mut()
                            .get_upvalue_at_mut(slot)
                            .set_location_value(value);
                    }
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
                }
                OpCode::OpClosure => {
                    // Safety: We know that OpClosure takes one arguments to which self.ip points,
                    //         because it is incremented after reading this opcode.
                    //         Also self.ip gets incremented after reading the constant so it will
                    //         point to the next opcode after this.
                    let function = unsafe { self.read_constant() };

                    if let Value::Function(function) = function {
                        let mut closure = Closure::new(function.clone());
                        let count = closure.upvalue_count();

                        for _ in 0..count {
                            let is_local = unsafe { self.read_index() } != 0;
                            let index = unsafe { self.read_index() } as usize;
                            let frame = self.frames.last_mut().unwrap();
                            let upvalue = if is_local {
                                let location = frame.get_slots() + index;
                                let location = UpvalueLocation::Stack(location);
                                self.capture_upvalue(location)
                            } else {
                                frame.get_closure().get_upvalue_at(index).clone()
                            };

                            closure.push_upvalue(upvalue);
                        }

                        self.stack.push(Value::Closure(closure));
                    } else {
                        panic!("Expected a function value.");
                    }
                }
                OpCode::OpCloseUpvalue => {
                    self.close_upvalues(self.stack.len() - 1);
                    self.stack.pop();
                }
            }
        }
    }

    fn capture_upvalue(&mut self, location: UpvalueLocation) -> ObjUpvalue {
        if let Some(upvalue) = self
            .open_upvalues
            .iter()
            .rev()
            .find(|v| v.get_location() == location)
            .cloned()
        {
            upvalue
        } else {
            let upvalue = ObjUpvalue::new(location);
            self.open_upvalues.push(upvalue.clone());
            upvalue
        }
    }

    fn close_upvalues(&mut self, last: usize) {
        // Should use Vec::drain_filter(...) here, but that's nightly-only at the time of writing.
        let mut i: usize = 0;
        while i < self.open_upvalues.len() {
            if let UpvalueLocation::Stack(s) = self.open_upvalues[i].get_location() {
                if s >= last {
                    let mut upvalue = self.open_upvalues.remove(i);
                    if let UpvalueLocation::Stack(index) = upvalue.get_location().clone() {
                        let val = self.stack[index].clone();
                        upvalue.set_location(UpvalueLocation::Heap(std::rc::Rc::new(val)));
                    } else {
                        panic!("Expected upvalue to be be located on stack.");
                    }
                } else {
                    i += 1;
                }
            } else {
                panic!("Expected location to be on stack!");
            }
        }
    }

    fn call_value(&mut self, callee: Value, arg_count: u8) -> bool {
        match callee {
            Value::Function(fun) => unreachable!("Functions are always wrapped in closures."),
            Value::Closure(closure) => self.call(closure, arg_count),
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

    fn call(&mut self, closure: Closure, arg_count: u8) -> bool {
        if arg_count as usize == closure.get_function().get_arity() {
            let frame = CallFrame::new(closure, 0, self.stack.len() - arg_count as usize - 1);
            self.frames.push(frame);
            true
        } else {
            self.runtime_error(
                format!(
                    "Expected {} arguments, but got {}.",
                    closure.get_function().get_arity(),
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
        let chunk = frame.get_closure().get_function().get_chunk();
        let ip = frame.get_ip();
        let code_unit = chunk.get_code_unit(ip);
        frame.inc_ip(1);
        code_unit.get_index()
    }

    /// Safety: It is only safe to call this function when self.ip is the index of an short value
    /// consisting of two consecutive indexes in self.chunk.
    unsafe fn read_short(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let chunk = frame.get_closure().get_function().get_chunk();
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
        let chunk = frame.get_closure().get_function().get_chunk();
        chunk.get_value_at_index(index)
    }

    /// Safety: It's only save to call this function when self.ip is the index of an opcode in
    ///         self.chunk.
    unsafe fn read_opcode(&mut self) -> OpCode {
        let frame = self.frames.last_mut().unwrap();
        let chunk = frame.get_closure().get_function().get_chunk();
        let ip = frame.get_ip();
        let code_unit = chunk.get_code_unit(ip);
        frame.inc_ip(1);
        code_unit.get_opcode()
    }

    fn runtime_error(&mut self, message: &str) {
        for frame in self.frames.iter().rev() {
            let function = frame.get_closure().get_function();
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
    closure: Closure,
    ip: usize,
    slots: usize,
}

impl CallFrame {
    pub fn new(closure: Closure, ip: usize, slots: usize) -> Self {
        Self { closure, ip, slots }
    }

    pub fn get_closure(&self) -> &Closure {
        &self.closure
    }

    pub fn get_closure_mut(&mut self) -> &mut Closure {
        &mut self.closure
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
