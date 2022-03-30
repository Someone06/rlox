use crate::chunk::{Chunk, OpCode, Value};

#[derive(PartialEq, Eq, Debug)]
pub enum InterpretResult {
    CompileError,
    RuntimeError,
}

pub struct VM {
    chunk: Chunk,
    ip: usize,
    stack: Vec<Value>,
}

impl VM {
    pub fn new(chunk: Chunk) -> Self {
        VM {
            chunk,
            ip: 0,
            stack: Vec::new(),
        }
    }

    pub fn interpret(&mut self) -> Result<Value, InterpretResult> {
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
                let _ = self.chunk.print_disassemble_instruction_unsafe(self.ip - 1);
            }

            match opcode {
                OpCode::OpReturn => {
                    let result = self
                        .stack
                        .pop()
                        .expect("Stack should never be empty when executing OpReturn.");
                    println!("{}", result);
                    return Ok(result);
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
                    let function = |a, b| {
                        if let (Value::Double(f1), Value::Double(f2)) = (a, b) {
                            Ok(Value::Double(f1 + f2))
                        } else {
                            Err(InterpretResult::RuntimeError)
                        }
                    };
                    self.binary_double_op(function)?;
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
            }
        }
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
        self.ip = 0;
    }

    /// Safety: It is only safe to call this function when self.ip is the index of an index in
    /// self.chunk.
    unsafe fn read_constant(&mut self) -> &Value {
        let code_unit = self.chunk.get_code_unit(self.ip);
        self.ip += 1;

        let index = code_unit.get_index();
        self.chunk.get_value_at_index(index)
    }

    /// Safety: It's only save to call this function when self.ip is the index of an opcode in
    ///         self.chunk.
    unsafe fn read_opcode(&mut self) -> OpCode {
        let code_unit = self.chunk.get_code_unit(self.ip);
        self.ip += 1;

        code_unit.get_opcode()
    }

    fn runtime_error(&mut self, message: &str) {
        eprintln!(
            "{}\n[line {}] in script",
            message,
            self.chunk.get_source_code_line(self.ip)
        );

        self.reset_stack();
    }

    #[cfg(debug_assertions)]
    fn print_stack(&self) {
        self.stack.iter().for_each(|value| print!("[{}]", value));
        println!();
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk::{ChunkBuilder, OpCode, Value};
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
            let chunk = $builder.build();
            let mut vm = VM::new(chunk);
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
