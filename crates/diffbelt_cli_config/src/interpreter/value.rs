use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ValueHolder {
    pub value: Value,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PrimitiveValue {
    String(Rc<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Map(Rc<RefCell<HashMap<PrimitiveValue, Value>>>),
    List(Rc<RefCell<Vec<Value>>>),
    String(Rc<str>),
    U64(u64),
}

impl Value {
    pub fn as_primitive_value(&self) -> Result<PrimitiveValue, ()> {
        match self {
            Value::String(s) => Ok(PrimitiveValue::String(s.clone())),
            _ => Err(()),
        }
    }
}
