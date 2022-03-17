use static_assertions::assert_eq_size;
use std::io::Write;

/// This enum represents all opcodes, that is the instruction set of the virtual machine.
/// We ensure that each opcode can be represented as a u8, to allow for a densely packed bytecode.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    OpConstant,
    OpReturn,
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let s: &str = match &self {
            OpCode::OpConstant => "OpConstant",
            OpCode::OpReturn => "OpReturn",
        };

        f.write_str(s)
    }
}

// Some opcodes require arguments in form of values (e.g. doubles or strings).
// Instead of storing these inline we have a separate pool for values in which we index.
// The indexes are stored inline in the instruction sequence.
#[derive(Clone, Copy)]
union CodeUnit {
    opcode: OpCode,
    index: u8,
}

// We want to fit code units in an Vec<u8> so, ensure that we have the right size.
assert_eq_size! {CodeUnit, u8}

impl CodeUnit {
    // Safety: A code unit eiter stores an opcode or an index, but not which one is stored.
    //         It is only safe to call this method if it is known (from external knowledge) that
    //         this code unit currently stores an opcode and not an index.
    unsafe fn get_opcode(&self) -> OpCode {
        self.opcode
    }

    // Safety: A code unit eiter stores an opcode or an index, but not which one is stored.
    //         It is only safe to call this method if it is known (from external knowledge) that
    //         this code unit currently stores an index and not an opcode.
    unsafe fn get_index(&self) -> u8 {
        self.index
    }
}

impl From<OpCode> for CodeUnit {
    fn from(opcode: OpCode) -> Self {
        CodeUnit { opcode }
    }
}

impl From<u8> for CodeUnit {
    fn from(index: u8) -> Self {
        CodeUnit { index }
    }
}

pub enum Value {
    Double(f64),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let s = match &self {
            Value::Double(f) => f.to_string(),
        };

        f.write_str(s.as_str())
    }
}

/// A chunk represents a sequence of instructions alongside their arguments.
pub struct Chunk {
    code: Vec<CodeUnit>,
    constants: Vec<Value>,
    lines: Vec<u32>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    pub fn write_opcode(&mut self, opcode: OpCode, line: u32) {
        self.code.push(CodeUnit::from(opcode));
        self.lines.push(line);
    }

    pub fn write_index(&mut self, index: u8) {
        self.code.push(CodeUnit::from(index));
        self.lines.push(
            *self
                .lines
                .last()
                .expect("First code unit cannot be an index."),
        );
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub fn print_dissasemble(&self, name: &str) -> std::io::Result<()> {
        self.disassemble(name, &mut std::io::stdout())
    }

    pub fn disassemble(&self, name: &str, writer: &mut impl Write) -> std::io::Result<()> {
        writeln!(writer, "== {} ==", name)?;

        let mut offset: usize = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset, writer)?;
        }

        Ok(())
    }

    /// Format: <offset> <opcode> <index> <value>
    /// Index and value are optional.
    fn disassemble_instruction(
        &self,
        offset: usize,
        writer: &mut impl Write,
    ) -> Result<usize, std::io::Error> {
        write!(writer, "{:04} ", offset)?;
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            write!(writer, "   | ")?;
        } else {
            write!(writer, "{:4} ", self.lines[offset])?;
        }

        let code_unit = self.code[offset];
        // Safety: The first code unit is assumed to be an instruction.
        //         For each instruction we know how many of the following code units are indexes.
        //         These are skipped by increasing the offset by
        //         (1 + <number of indexes following the current instruction>).
        //         So the offset once again points to an OpCode.
        let opcode = unsafe { code_unit.get_opcode() };

        match opcode {
            OpCode::OpConstant => self.constant_instruction(opcode, offset, writer),
            OpCode::OpReturn => self.simple_instruction(opcode, offset, writer),
            _ => writeln!(writer, "Unknown opcode: {}", opcode as u8).map(|_| offset + 1),
        }
    }

    fn constant_instruction(
        &self,
        opcode: OpCode,
        offset: usize,
        writer: &mut impl Write,
    ) -> Result<usize, std::io::Error> {
        let code_unit = self.code[offset + 1];

        // Safety: We know that the instruction at offset is a constant instruction.
        // That instruction requires exactly one index, the code unit at offset + 1 has to be an
        // index.
        let index = unsafe { code_unit.get_index() };
        let value = &self.constants[index as usize];
        writeln!(writer, "{:-16} {:4} '{}'", opcode, index, value).map(|_| offset + 2)
    }

    fn simple_instruction(
        &self,
        opcode: OpCode,
        offset: usize,
        writer: &mut impl Write,
    ) -> Result<usize, std::io::Error> {
        writeln!(writer, "{}", opcode).map(|_| offset + 1)
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk::{Chunk, OpCode, Value};

    #[test]
    fn disassemble_constant() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::OpConstant, 0);
        chunk.write_index(0);
        chunk.add_constant(Value::Double(2.0));

        let mut buffer: Vec<u8> = Vec::new();
        chunk.disassemble("test chunk", &mut buffer).unwrap();

        let result = std::str::from_utf8(&buffer).expect("Just wrote a string into the buffer");
        assert_eq!(result, "== test chunk ==\n0000    0 OpConstant    0 '2'\n")
    }

    #[test]
    fn disassemble_return() {
        let mut chunk = Chunk::new();
        chunk.write_opcode(OpCode::OpReturn, 0);

        let mut buffer: Vec<u8> = Vec::new();
        chunk.disassemble("test chunk", &mut buffer).unwrap();

        let result = std::str::from_utf8(&buffer).expect("Just wrote a string into the buffer");
        assert_eq!(result, "== test chunk ==\n0000    0 OpReturn\n")
    }
}
