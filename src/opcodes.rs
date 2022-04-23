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
    OpGetUpvalue,
    OpSetUpvalue,
    OpJump,
    OpJumpIfFalse,
    OpLoop,
    OpCall,
    OpClosure,
    OpCloseUpvalue,
    OpClass,
    OpGetProperty,
    OpSetProperty,
    OpMethod,
}

pub struct IndexesPerOpCode {
    map: enum_map::EnumMap<OpCode, u8>,
}

/// This struct is used to get how many arguments each Opcode required.
/// Each Opcode requires a fix ammount of arguments.
/// The only exeption is OpCode::OpClosure, which requires a variable number of
/// arguments.
impl IndexesPerOpCode {
    pub fn new() -> Self {
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
            OpCode::OpGetUpvalue => 1,
            OpCode::OpSetUpvalue => 1,
            OpCode::OpJump => 2,
            OpCode::OpJumpIfFalse => 2,
            OpCode::OpLoop => 2,
            OpCode::OpCall => 1,
            OpCode::OpClosure => u8::MAX,
            OpCode::OpCloseUpvalue => 0,
            OpCode::OpClass => 1,
            OpCode::OpGetProperty => 1,
            OpCode::OpSetProperty => 1,
            OpCode::OpMethod => 1,
        };

        IndexesPerOpCode { map }
    }

    pub fn get(&self, opcode: OpCode) -> u8 {
        self.map[opcode]
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
