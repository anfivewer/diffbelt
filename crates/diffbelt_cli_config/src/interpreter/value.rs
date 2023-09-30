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

#[derive(Debug, Clone)]
pub enum Value {
    Map(RefCell<HashMap<PrimitiveValue, Value>>),
    List(Vec<Value>),
    String(Rc<str>),
    U64(u64),
}
