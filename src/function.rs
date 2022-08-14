use std::cell::RefCell;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::chunk::{Chunk, ChunkBuilder};
use crate::intern_string::Symbol;
use crate::value::Value;

pub struct Function {
    inner: Rc<FunctionInner>,
}

impl Function {
    fn new(
        name: Option<Symbol>,
        arity: usize,
        chunk: Chunk,
        upvalue_count: usize,
        kind: FunctionType,
    ) -> Self {
        let inner = FunctionInner::new(name, arity, chunk, upvalue_count, kind);
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

    pub fn get_upvalue_count(&self) -> usize {
        self.inner.get_upvalue_count()
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
    upvalue_count: usize,
}

impl FunctionInner {
    fn new(
        name: Option<Symbol>,
        arity: usize,
        chunk: Chunk,
        upvalue_count: usize,
        kind: FunctionType,
    ) -> Self {
        Self {
            arity,
            name,
            chunk,
            kind,
            upvalue_count,
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

    fn get_upvalue_count(&self) -> usize {
        self.upvalue_count
    }
}

impl Display for FunctionInner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
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
    upvalue_count: usize,
}

impl FunctionBuilder {
    pub fn new(name: Option<Symbol>, arity: usize, kind: FunctionType) -> Self {
        FunctionBuilder {
            name,
            arity,
            kind,
            builder: ChunkBuilder::new(),
            upvalue_count: 0,
        }
    }

    pub fn get_name(&self) -> Option<&Symbol> {
        self.name.as_ref()
    }

    pub fn get_arity(&self) -> usize {
        self.arity
    }

    pub fn get_kind(&self) -> FunctionType {
        self.kind
    }

    pub fn set_kind(&mut self, kind: FunctionType) {
        self.kind = kind;
    }

    pub fn inc_arity(&mut self, amount: usize) {
        self.arity += amount;
    }

    pub fn set_name(&mut self, name: Symbol) {
        self.name = Some(name);
    }

    pub fn get_upvalue_count(&self) -> usize {
        self.upvalue_count
    }

    pub fn inc_upvalue_count(&mut self) {
        self.upvalue_count += 1;
    }

    pub fn build(self) -> Function {
        Function::new(
            self.name,
            self.arity,
            self.builder.build(),
            self.upvalue_count,
            self.kind,
        )
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
    Method,
    Initializer,
}

impl Display for FunctionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Copy, Clone)]
pub struct NativeFunction {
    function: fn(args: &[Value]) -> Value,
    arity: usize,
}

impl PartialEq for NativeFunction {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(
            self.function as *const fn(&[Value]) -> Value,
            other.function as *const _,
        )
    }
}

impl Eq for NativeFunction {}

impl Debug for NativeFunction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("<native fn>")
    }
}

impl NativeFunction {
    pub fn new(function: fn(&[Value]) -> Value, arity: usize) -> Self {
        NativeFunction { function, arity }
    }

    pub fn call(&self, args: &[Value]) -> Value {
        (self.function)(args)
    }

    pub fn get_arity(&self) -> usize {
        self.arity
    }
}

pub fn clock(_: &[Value]) -> Value {
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    Value::Double(since_the_epoch.as_secs_f64())
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Closure {
    function: Function,
    upvalues: Vec<ObjUpvalue>,
}

impl Closure {
    pub fn new(function: Function) -> Self {
        Closure {
            function,
            upvalues: Vec::new(),
        }
    }

    pub fn get_function(&self) -> &Function {
        &self.function
    }

    pub fn push_upvalue(&mut self, value: ObjUpvalue) {
        self.upvalues.push(value);
    }

    pub fn get_upvalue_at(&self, index: usize) -> &ObjUpvalue {
        &self.upvalues[index]
    }

    pub fn get_upvalue_at_mut(&mut self, index: usize) -> &mut ObjUpvalue {
        &mut self.upvalues[index]
    }

    pub fn upvalue_count(&self) -> usize {
        self.function.get_upvalue_count()
    }
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.function)
    }
}

#[derive(Clone, Debug)]
pub enum UpvalueLocation {
    Stack(usize),
    Heap(Rc<Value>),
}

impl PartialEq for UpvalueLocation {
    fn eq(&self, other: &Self) -> bool {
        if let (UpvalueLocation::Stack(a), UpvalueLocation::Stack(b)) = (&self, &other) {
            a == b
        } else if let (UpvalueLocation::Heap(a), UpvalueLocation::Heap(b)) = (&self, &other) {
            Rc::ptr_eq(a, b)
        } else {
            false
        }
    }
}

impl Eq for UpvalueLocation {}

#[derive(Clone, PartialEq, Eq, Debug)]
struct ObjUpvalueInner {
    location: UpvalueLocation,
}

impl ObjUpvalueInner {
    fn new(location: UpvalueLocation) -> Self {
        ObjUpvalueInner { location }
    }

    fn get_location(&self) -> &UpvalueLocation {
        &self.location
    }

    fn get_location_mut(&mut self) -> &mut UpvalueLocation {
        &mut self.location
    }

    fn set_location(&mut self, location: UpvalueLocation) {
        self.location = location;
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ObjUpvalue {
    inner: Rc<RefCell<ObjUpvalueInner>>,
}

impl ObjUpvalue {
    pub fn new(location: UpvalueLocation) -> Self {
        ObjUpvalue {
            inner: Rc::new(RefCell::new(ObjUpvalueInner::new(location))),
        }
    }

    pub fn get_location(&self) -> UpvalueLocation {
        self.inner.deref().borrow().get_location().clone()
    }

    pub fn set_location_value(&mut self, value: Value) {
        let mut borrow = self.inner.deref().borrow_mut();
        let location = borrow.get_location_mut();
        match location {
            UpvalueLocation::Stack(_) => panic!("We assume that the location is on heap."),
            UpvalueLocation::Heap(_) => *location = UpvalueLocation::Heap(Rc::new(value)),
        }
    }

    pub fn set_location(&mut self, location: UpvalueLocation) {
        self.inner.deref().borrow_mut().set_location(location);
    }
}
