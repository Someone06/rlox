use ::weak_table::WeakHashSet;

use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak};

#[derive(Clone)]
pub struct Symbol {
    intern: Rc<String>,
}

impl Symbol {
    fn new(intern: Rc<String>) -> Self {
        Symbol { intern }
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Symbol) -> bool {
        Rc::ptr_eq(&self.intern, &other.intern)
    }
}

impl Eq for Symbol {}

impl Hash for Symbol {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.intern.hash(state);
    }
}

impl Deref for Symbol {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        self.intern.deref()
    }
}

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(self.intern.deref())
    }
}

#[derive(Default)]
pub struct SymbolTable {
    pool: WeakHashSet<Weak<String>>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: String) -> Symbol {
        if let Some(rc) = self.pool.get(&name) {
            Symbol::new(rc)
        } else {
            let rc = Rc::new(name);
            self.pool.insert(rc.clone());
            Symbol::new(rc)
        }
    }
}
