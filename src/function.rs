use crate::chunk::{Chunk, ChunkBuilder};
use crate::intern_string::Symbol;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub struct Function {
    inner: Rc<FunctionInner>,
}

impl Function {
    fn new(name: Option<Symbol>, arity: usize, chunk: Chunk, kind: FunctionType) -> Self {
        let inner = FunctionInner::new(name, arity, chunk, kind);
        Function {
            inner: Rc::new(inner),
        }
    }

    pub fn get_name(&self) -> Option<&Symbol> {
        self.inner.get_name()
    }

    pub fn get_arity(&self) -> usize {
        self.inner.get_arity()
    }

    pub fn get_chunk(&self) -> &Chunk {
        self.inner.get_chunk()
    }

    pub fn get_kind(&self) -> FunctionType {
        self.inner.get_kind()
    }
}

impl Clone for Function {
    fn clone(&self) -> Self {
        Function {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl PartialEq for Function {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for Function {}

impl Display for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

impl Debug for Function {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.inner)
    }
}

pub struct FunctionInner {
    arity: usize,
    name: Option<Symbol>,
    chunk: Chunk,
    kind: FunctionType,
}

impl FunctionInner {
    fn new(name: Option<Symbol>, arity: usize, chunk: Chunk, kind: FunctionType) -> Self {
        Self {
            arity,
            name,
            chunk,
            kind,
        }
    }

    fn get_arity(&self) -> usize {
        self.arity
    }

    fn get_name(&self) -> Option<&Symbol> {
        self.name.as_ref()
    }

    fn get_chunk(&self) -> &Chunk {
        &self.chunk
    }

    fn get_kind(&self) -> FunctionType {
        self.kind
    }
}

impl std::fmt::Display for FunctionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<fn {}>",
            self.get_name().map_or("<script>", |s| s.as_str())
        )
    }
}

pub struct FunctionBuilder {
    name: Option<Symbol>,
    arity: usize,
    kind: FunctionType,
    builder: ChunkBuilder,
}

impl FunctionBuilder {
    pub fn new(name: Option<Symbol>, arity: usize, kind: FunctionType) -> Self {
        FunctionBuilder {
            name,
            arity,
            kind,
            builder: ChunkBuilder::new(),
        }
    }
    
    pub fn get_name(&self) -> Option<&Symbol> {
        self.name.as_ref()
    }

    pub fn build(self) -> Function {
        Function::new(self.name, self.arity, self.builder.build(), self.kind)
    }
}

impl Deref for FunctionBuilder {
    type Target = ChunkBuilder;

    fn deref(&self) -> &Self::Target {
        &self.builder
    }
}

impl DerefMut for FunctionBuilder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.builder
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FunctionType {
    Function,
    Script,
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
