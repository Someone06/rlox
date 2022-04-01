use ::std::io::Write;

use crate::intern_string::Symbol;

/// This enum represents all opcodes, that is the instruction set of the virtual machine.
/// We ensure that each opcode can be represented as a u8, to allow for a densely packed bytecode.
#[derive(Copy, Clone, PartialEq, Eq, Debug, ::enum_map::Enum)]
#[repr(u8)]
pub enum OpCode {
    OpConstant,
    OpNil,
    OpTrue,
    OpFalse,
    OpNegate,
    OpAdd,
    OpSubtract,
    OpMultiply,
    OpDivide,
    OpNot,
    OpEqual,
    OpGreater,
    OpLess,
    OpReturn,
    OpPrint,
    OpPop,
    OpDefineGlobal,
    OpGetGlobal,
    OpSetGlobal,
    OpGetLocal,
    OpSetLocal,
}

struct IndexesPerOpCode {
    map: ::enum_map::EnumMap<OpCode, u8>,
}

impl IndexesPerOpCode {
    fn new() -> Self {
        let map = ::enum_map::enum_map! {
            OpCode::OpConstant => 1,
            OpCode::OpNil => 0,
            OpCode::OpTrue => 0,
            OpCode::OpFalse => 0,
            OpCode::OpNegate => 0,
            OpCode::OpAdd => 0,
            OpCode::OpSubtract => 0,
            OpCode::OpMultiply => 0,
            OpCode::OpDivide => 0,
            OpCode::OpNot => 0,
            OpCode::OpEqual => 0,
            OpCode::OpGreater => 0,
            OpCode::OpLess => 0,
            OpCode::OpReturn => 0,
            OpCode::OpPrint => 0,
            OpCode::OpPop => 0,
            OpCode::OpDefineGlobal => 1,
            OpCode::OpGetGlobal => 1,
            OpCode::OpSetGlobal => 1,
            OpCode::OpGetLocal => 1,
            OpCode::OpSetLocal => 1,
        };

        IndexesPerOpCode { map }
    }

    fn get(&self, opcode: OpCode) -> u8 {
        self.map[opcode]
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}

/// Some opcodes require arguments in form of values (e.g. doubles or strings).
/// Instead of storing these inline we have a separate pool for values in which we index.
/// The indexes are stored inline in the instruction sequence.
#[derive(Clone, Copy)]
pub union CodeUnit {
    opcode: OpCode,
    index: u8,
}

impl CodeUnit {
    /// Safety: A code unit eiter stores an opcode or an index, but not which one is stored.
    ///         It is only safe to call this method if it is known (from external knowledge) that
    ///         this code unit currently stores an opcode and not an index.
    pub unsafe fn get_opcode(&self) -> OpCode {
        self.opcode
    }

    /// Safety: A code unit eiter stores an opcode or an index, but not which one is stored.
    ///         It is only safe to call this method if it is known (from external knowledge) that
    ///         this code unit currently stores an index and not an opcode.
    pub unsafe fn get_index(&self) -> u8 {
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

// We want to fit code units in an Vec<u8> so, ensure that we have the right size.
::static_assertions::assert_eq_size! {CodeUnit, u8}

/// This enum represents all constants that can be stored in the constant pool.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Double(f64),
    String(Symbol),
    Nil,
}

impl Value {
    pub fn is_falsey(&self) -> bool {
        matches!(self, Value::Nil | Value::Bool(false))
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        let s = match &self {
            Value::Bool(b) => b.to_string(),
            Value::Double(f) => f.to_string(),
            Value::String(s) => s.to_string(),
            Value::Nil => String::from("Nil"),
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

// Public API of a Chunk.
impl Chunk {
    /// Returns the code unit located at the given instruction index.
    /// Could be an opcode or an index.
    /// Panics if the given instruction index is out of range.
    pub fn get_code_unit(&self, instruction_index: usize) -> CodeUnit {
        self.code[instruction_index]
    }

    /// Returns the number of the source code line that corresponds to the instruction located at the
    /// given instruction index.
    /// Panics if the given instruction index is out of range.
    pub fn get_source_code_line(&self, instruction_index: usize) -> u32 {
        self.lines[instruction_index]
    }

    /// Returns a reference to the value located at the given index.
    /// Panics if the given index is out of range.
    pub fn get_value_at_index(&self, index: u8) -> &Value {
        &self.constants[index as usize]
    }

    /// Prints a disassemble of the chunk to stdout.
    /// Name is the name of this chunk.
    pub fn print_disassemble(&self, name: &str) -> std::io::Result<()> {
        self.disassemble(name, &mut std::io::stdout())
    }

    /// Writes a disassemble of this chunk to the given writer.
    /// Name is the name of this chunk.
    pub fn disassemble(&self, name: &str, writer: &mut impl Write) -> std::io::Result<()> {
        writeln!(writer, "== {} ==", name)?;

        let mut offset: usize = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset, writer)?;
        }

        Ok(())
    }

    /// Writes a disassemble of the opcode at the given offset to the given writer.
    /// Safety: Requires that offset points to an opcode.
    pub unsafe fn print_disassemble_instruction_unsafe(
        &self,
        offset: usize,
    ) -> Result<(), std::io::Error> {
        self.disassemble_instruction_unsafe(offset, &mut std::io::stdout())
    }

    /// Writes a disassemble of the opcode at the given offset to the given writer.
    /// Safety: Requires that offset points to an opcode.
    pub unsafe fn disassemble_instruction_unsafe(
        &self,
        offset: usize,
        writer: &mut impl Write,
    ) -> Result<(), std::io::Error> {
        self.disassemble_instruction(offset, writer).map(|_| ())
    }
}

// Private API of a chunk.
impl Chunk {
    fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }

    fn write_opcode(&mut self, opcode: OpCode, line: u32) {
        self.code.push(CodeUnit::from(opcode));
        self.lines.push(line);
    }

    fn write_index(&mut self, index: u8) {
        self.code.push(CodeUnit::from(index));
        self.lines.push(
            *self
                .lines
                .last()
                .expect("First code unit cannot be an index."),
        );
    }

    fn add_constant(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    fn finish(&mut self) {
        self.code.shrink_to_fit();
        self.constants.shrink_to_fit();
        self.lines.shrink_to_fit();
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
            OpCode::OpConstant
            | OpCode::OpDefineGlobal
            | OpCode::OpGetGlobal
            | OpCode::OpSetGlobal => self.constant_instruction(opcode, offset, writer),
            OpCode::OpGetLocal | OpCode::OpSetLocal => {
                self.byte_instruction(opcode, offset, writer)
            }
            OpCode::OpReturn
            | OpCode::OpPrint
            | OpCode::OpPop
            | OpCode::OpEqual
            | OpCode::OpLess
            | OpCode::OpGreater
            | OpCode::OpNegate
            | OpCode::OpNot
            | OpCode::OpAdd
            | OpCode::OpSubtract
            | OpCode::OpMultiply
            | OpCode::OpDivide
            | OpCode::OpTrue
            | OpCode::OpFalse
            | OpCode::OpNil => self.simple_instruction(opcode, offset, writer),
        }
    }

    fn byte_instruction(
        &self,
        opcode: OpCode,
        offset: usize,
        writer: &mut impl Write,
    ) -> Result<usize, std::io::Error> {
        let code_unit = self.code[offset + 1];

        // Safety: We know that the instruction at offset is a byte instruction.
        // That instruction requires exactly one index, so the code unit at offset + 1 has to be an
        // index.
        let index = unsafe { code_unit.get_index() };
        writeln!(writer, "{:-16} {:4}", opcode, index).map(|_| offset + 2)
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

/// ChunkBuilder is used to incrementally build a Chunk.
/// It ensures that the Chunk is in a valid state once it is build.
pub struct ChunkBuilder {
    chunk: Chunk,
    required_indexes: u8,
    max_index: Option<usize>,
    constant_index: Option<usize>,
    indexes_per_op: IndexesPerOpCode,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        ChunkBuilder {
            chunk: Chunk::new(),
            required_indexes: 0,
            max_index: None,
            constant_index: None,
            indexes_per_op: IndexesPerOpCode::new(),
        }
    }

    pub fn write_opcode(&mut self, opcode: OpCode, line: u32) {
        if self.required_indexes == 0 {
            self.chunk.write_opcode(opcode, line);
            self.required_indexes = self.indexes_per_op.get(opcode);
        } else {
            panic!("Requiring an index next.");
        }
    }

    // In case we will support > 255 constants, make sure to take a larger index here and break it
    // up into multiple u8 which can be written individually.
    pub fn write_index(&mut self, index: u8) {
        if self.required_indexes != 0 {
            self.chunk.write_index(index);
            self.required_indexes -= 1;
            if self.max_index.is_none() || self.max_index.unwrap() < (index as usize) {
                self.max_index = Some(index as usize);
            }
        } else {
            panic!("Requiring an opcode next.")
        }
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        let index = self.chunk.add_constant(value);
        self.constant_index = Some(index);
        index
    }

    pub fn build(mut self) -> Chunk {
        if self.required_indexes == 0 && self.max_index == self.constant_index {
            self.chunk.finish();
            self.chunk
        } else if self.required_indexes != 0 {
            panic!("Still requiring an index.");
        } else {
            panic!("Did not get the right amount of constants.");
        }
    }

    /// Writes a disassemble of the chunk that's been build so far to stdout.
    /// Name is the name of this chunk.
    pub fn print_disassemble(&self, name: &str) -> std::io::Result<()> {
        self.chunk.print_disassemble(name)
    }
}

#[cfg(test)]
mod tests {
    use crate::chunk::{ChunkBuilder, OpCode, Value};

    #[test]
    fn disassemble_constant() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_opcode(OpCode::OpConstant, 0);
        chunk_builder.write_index(0);
        chunk_builder.add_constant(Value::Double(2.0));

        let mut buffer: Vec<u8> = Vec::new();
        chunk_builder
            .build()
            .disassemble("test chunk", &mut buffer)
            .unwrap();

        let result = std::str::from_utf8(&buffer).expect("Just wrote a string into the buffer");
        assert_eq!(result, "== test chunk ==\n0000    0 OpConstant    0 '2'\n")
    }

    macro_rules! test_stack_only_op {
        ($op:expr) => {{
            let op = $op;
            let mut chunk_builder = ChunkBuilder::new();
            chunk_builder.write_opcode(op, 0);
            let mut buffer: Vec<u8> = Vec::new();
            chunk_builder
                .build()
                .disassemble("test chunk", &mut buffer)
                .unwrap();
            let result = std::str::from_utf8(&buffer).expect("Just wrote a string into the buffer");
            assert_eq!(result, format!("== test chunk ==\n0000    0 {}\n", op));
        }};
    }

    #[test]
    fn disassemble_stack_only_op() {
        test_stack_only_op!(OpCode::OpAdd);
        test_stack_only_op!(OpCode::OpSubtract);
        test_stack_only_op!(OpCode::OpNegate);
        test_stack_only_op!(OpCode::OpMultiply);
        test_stack_only_op!(OpCode::OpDivide);
        test_stack_only_op!(OpCode::OpReturn);
    }

    #[test]
    #[should_panic]
    fn require_opcode_first() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_index(0);
        chunk_builder.write_opcode(OpCode::OpReturn, 0);
        chunk_builder.add_constant(Value::Double(0.0));
        let _ = chunk_builder.build();
    }

    #[test]
    #[should_panic]
    fn require_index() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_opcode(OpCode::OpConstant, 0);
        chunk_builder.write_opcode(OpCode::OpConstant, 1);
        chunk_builder.write_index(0);
        chunk_builder.write_index(1);
        chunk_builder.add_constant(Value::Double(0.0));
        chunk_builder.add_constant(Value::Double(1.0));
        let _ = chunk_builder.build();
    }

    #[test]
    #[should_panic]
    fn too_many_indexes() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_opcode(OpCode::OpConstant, 0);
        chunk_builder.write_index(0);
        chunk_builder.add_constant(Value::Double(0.0));
        chunk_builder.write_index(1);
        chunk_builder.add_constant(Value::Double(1.0));
        chunk_builder.write_opcode(OpCode::OpReturn, 1);
        let _ = chunk_builder.build();
    }

    #[test]
    #[should_panic]
    fn not_enough_constants() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_opcode(OpCode::OpConstant, 0);
        chunk_builder.write_index(0);
        let _ = chunk_builder.build();
    }

    #[test]
    #[should_panic]
    fn too_many_constants() {
        let mut chunk_builder = ChunkBuilder::new();
        chunk_builder.write_opcode(OpCode::OpConstant, 0);
        chunk_builder.write_index(0);
        chunk_builder.add_constant(Value::Double(0.0));
        chunk_builder.add_constant(Value::Double(1.0));
        let _ = chunk_builder.build();
    }
}
