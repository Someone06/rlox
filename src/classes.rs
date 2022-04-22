use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

use crate::intern_string::Symbol;
use crate::value::Value;

#[derive(Debug)]
pub struct Clazz {
    name: Symbol,
}

impl Clazz {
    pub fn new(name: Symbol) -> Self {
        Clazz { name }
    }

    pub fn get_name(&self) -> &Symbol {
        &self.name
    }
}

impl std::fmt::Display for Clazz {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.write_str(self.name.to_string().as_str())
    }
}

#[derive(Clone, Debug)]
pub struct ClazzRef {
    clazz: Rc<RefCell<Clazz>>,
}

impl ClazzRef {
    pub fn new(clazz: Clazz) -> Self {
        ClazzRef {
            clazz: Rc::new(RefCell::new(clazz)),
        }
    }

    pub fn get_clazz(&self) -> std::cell::Ref<'_, Clazz> {
        self.clazz.deref().borrow()
    }

    pub fn get_clazz_mut(&mut self) -> std::cell::RefMut<'_, Clazz> {
        self.clazz.deref().borrow_mut()
    }
}

impl From<Clazz> for ClazzRef {
    fn from(clazz: Clazz) -> Self {
        ClazzRef {
            clazz: Rc::new(RefCell::new(clazz)),
        }
    }
}

impl PartialEq for ClazzRef {
    fn eq(&self, other: &ClazzRef) -> bool {
        Rc::ptr_eq(&self.clazz, &other.clazz)
    }
}

impl Eq for ClazzRef {}

impl std::fmt::Display for ClazzRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.get_clazz())
    }
}

#[derive(Debug)]
pub struct Instance {
    clazz: ClazzRef,
    fields: HashMap<Symbol, Value>,
}

impl Instance {
    pub fn new(clazz: ClazzRef) -> Self {
        Instance {
            clazz,
            fields: HashMap::new(),
        }
    }

    pub fn get_value(&self, property: &Symbol) -> Option<&Value> {
        self.fields.get(property)
    }

    pub fn set_value(&mut self, name: Symbol, value: Value) {
        self.fields.insert(name, value);
    }
}

impl std::fmt::Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{} instance", self.clazz)
    }
}

#[derive(Clone, Debug)]
pub struct InstanceRef {
    instance: Rc<RefCell<Instance>>,
}

impl InstanceRef {
    pub fn new(instance: Instance) -> Self {
        InstanceRef {
            instance: Rc::new(RefCell::new(instance)),
        }
    }

    pub fn get_instance(&self) -> std::cell::Ref<'_, Instance> {
        self.instance.deref().borrow()
    }

    pub fn get_instance_mut(&mut self) -> std::cell::RefMut<'_, Instance> {
        self.instance.deref().borrow_mut()
    }
}

impl From<Instance> for InstanceRef {
    fn from(instance: Instance) -> Self {
        InstanceRef {
            instance: Rc::new(RefCell::new(instance)),
        }
    }
}

impl From<ClazzRef> for InstanceRef {
    fn from(clazz: ClazzRef) -> Self {
        InstanceRef {
            instance: Rc::new(RefCell::new(Instance::new(clazz))),
        }
    }
}

impl PartialEq for InstanceRef {
    fn eq(&self, other: &InstanceRef) -> bool {
        Rc::ptr_eq(&self.instance, &other.instance)
    }
}

impl Eq for InstanceRef {}

impl std::fmt::Display for InstanceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.get_instance())
    }
}
