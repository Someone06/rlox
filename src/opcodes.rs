/// This enum represents all opcodes, that is the instruction set of the virtual machine.
/// We ensure that each opcode can be represented as a u8, to allow for a densely packed bytecode.
#[derive(Copy, Clone, PartialEq, Eq, Debug, ::enum_map::Enum)]
#[repr(u8)]
pub enum OpCode {
    Constant,
    Nil,
    True,
    False,
    Negate,
    Add,
    Subtract,
    Multiply,
    Divide,
    Not,
    Equal,
    Greater,
    Less,
    Return,
    Print,
    Pop,
    DefineGlobal,
    GetGlobal,
    SetGlobal,
    GetLocal,
    SetLocal,
    GetUpvalue,
    SetUpvalue,
    Jump,
    JumpIfFalse,
    Loop,
    Call,
    Closure,
    CloseUpvalue,
    Class,
    GetProperty,
    SetProperty,
    Method,
    Invoke,
    Inherit,
    GetSuper,
    SuperInvoke,
}

pub struct IndexesPerOpCode {
    map: enum_map::EnumMap<OpCode, u8>,
}

/// This struct is used to get how many arguments each Opcode required.
/// Each Opcode requires a fix amount of arguments.
/// The only exception is OpCode::OpClosure, which requires a variable number of
/// arguments.
impl IndexesPerOpCode {
    pub fn new() -> Self {
        let map = enum_map::enum_map! {
            OpCode::Constant => 1,
            OpCode::Nil => 0,
            OpCode::True => 0,
            OpCode::False => 0,
            OpCode::Negate => 0,
            OpCode::Add => 0,
            OpCode::Subtract => 0,
            OpCode::Multiply => 0,
            OpCode::Divide => 0,
            OpCode::Not => 0,
            OpCode::Equal => 0,
            OpCode::Greater => 0,
            OpCode::Less => 0,
            OpCode::Return => 0,
            OpCode::Print => 0,
            OpCode::Pop => 0,
            OpCode::DefineGlobal => 1,
            OpCode::GetGlobal => 1,
            OpCode::SetGlobal => 1,
            OpCode::GetLocal => 1,
            OpCode::SetLocal => 1,
            OpCode::GetUpvalue => 1,
            OpCode::SetUpvalue => 1,
            OpCode::Jump => 2,
            OpCode::JumpIfFalse => 2,
            OpCode::Loop => 2,
            OpCode::Call => 1,
            OpCode::Closure => u8::MAX,
            OpCode::CloseUpvalue => 0,
            OpCode::Class => 1,
            OpCode::GetProperty => 1,
            OpCode::SetProperty => 1,
            OpCode::Method => 1,
            OpCode::Invoke => 2,
            OpCode::Inherit => 0,
            OpCode::GetSuper => 1,
            OpCode::SuperInvoke => 2,
        };

        IndexesPerOpCode { map }
    }

    pub fn get(&self, opcode: OpCode) -> u8 {
        self.map[opcode]
    }
}

impl std::fmt::Display for OpCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{:?}", self)
    }
}
