use ::weak_table::WeakHashSet;

use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::{Rc, Weak};

#[derive(Clone, Debug)]
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

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::ops::Deref;

    use crate::intern_string::{Symbol, SymbolTable};

    #[test]
    fn it_works() {
        let mut table = SymbolTable::new();
        let msg = "Hello world!";
        let symbol = table.intern(String::from(msg));
        assert_eq!(msg, *symbol);
    }

    #[test]
    fn more_strings() {
        let mut table = SymbolTable::new();
        let strs = vec!["Hello", "42", "1337", "\"'$$%&\"", "World"];
        let strings = strs
            .iter()
            .map(|s| String::from(*s))
            .collect::<Vec<String>>();
        let interned = strings
            .iter()
            .map(|s| table.intern(s.clone()))
            .collect::<HashSet<Symbol>>();
        assert_eq!(strs.len(), interned.len());

        assert_eq!(
            strs.iter()
                .rev()
                .map(|s| String::from(*s))
                .map(|s| table.intern(s))
                .collect::<HashSet<Symbol>>(),
            interned
        );

        assert_eq!(
            interned
                .iter()
                .map(|s| s.deref().clone())
                .collect::<HashSet<String>>(),
            strings.iter().cloned().collect::<HashSet<String>>()
        );
    }
}
