use std::collections::HashMap;

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
}