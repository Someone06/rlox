use crate::function::{Closure, Function, NativeFunction};
use crate::intern_string::Symbol;

/// This enum represents all constants that can be stored in the constant pool.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Double(f64),
    String(Symbol),
    Function(Function),
    NativeFunction(NativeFunction),
    Closure(Closure),
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
            Value::Function(f) => f.to_string(),
            Value::NativeFunction(_) => String::from("<native fn>"),
            Value::Closure(c) => c.get_function().to_string(),
            Value::Nil => String::from("Nil"),
        };

        f.write_str(s.as_str())
    }
}
