use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ValueHolder {
    pub value: Value,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum PrimitiveValue {
    String(Rc<str>),
}

impl PrimitiveValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            PrimitiveValue::String(s) => Some(s.deref()),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Bool(bool),
    Map(Rc<RefCell<HashMap<PrimitiveValue, Value>>>),
    List(Rc<RefCell<Vec<Value>>>),
    String(Rc<str>),
    Bytes(Rc<[u8]>),
    U64(u64),
    I64(i64),
    F64(f64),
}

impl Value {
    pub fn is_none(&self) -> bool {
        if let Self::None = self {
            true
        } else {
            false
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.deref()),
            _ => None,
        }
    }

    pub fn as_rc_str(&self) -> Option<Rc<str>> {
        match self {
            Value::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&Rc<RefCell<HashMap<PrimitiveValue, Value>>>> {
        match self {
            Value::Map(map) => Some(map),
            _ => None,
        }
    }

    pub fn as_primitive_value(&self) -> Result<PrimitiveValue, ()> {
        match self {
            Value::String(s) => Ok(PrimitiveValue::String(s.clone())),
            _ => Err(()),
        }
    }
}
