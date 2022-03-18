use crate::chunk::{Chunk, OpCode, Value};

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

    pub fn interpret(&mut self) -> Result<(), InterpretResult> {
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
                    println!(
                        "{}",
                        self.stack
                            .pop()
                            .expect("Stack should never be empty when executing OpReturn.")
                    );
                    return Ok(());
                }
                OpCode::OpConstant => {
                    // Safety: We know that OpConstant takes one arguments to which self.ip points,
                    //         because it is incremented after reading this opcode.
                    //         Also self.ip gets incremented after reading the constant so it will
                    //         point to the next opcode after this.
                    let value = unsafe { self.read_constant() }.clone();
                    self.stack.push(value);
                }
            }
        }
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

    #[cfg(debug_assertions)]
    fn print_stack(&self) {
        self.stack.iter().for_each(|value| print!("[{}]", value));
        println!();
    }
}
