use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct ValueHolder {
    pub value: Value,
}

#[derive(Debug, Clone)]
pub enum PrimitiveValue {
    String(String),
}

#[derive(Debug, Clone)]
pub enum Value {
    Map(HashMap<PrimitiveValue, Value>),
    List(Vec<Value>),
    String(Rc<str>),
    U64(u64),
}