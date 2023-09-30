use crate::interpreter::value::{PrimitiveValue, Value, ValueHolder};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct VarDef {
    pub name: String,
}

impl VarDef {
    pub fn unknown() -> Self {
        VarDef {
            name: String::with_capacity(0),
        }
    }

    pub fn anonymous_string() -> Self {
        VarDef {
            name: String::with_capacity(0),
        }
    }

    pub fn anonymous_u64() -> Self {
        VarDef {
            name: String::with_capacity(0),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Var {
    pub def: VarDef,
    pub value: Option<ValueHolder>,
}

impl Var {
    pub fn new_string(value: Rc<str>) -> Self {
        Var {
            def: VarDef::anonymous_string(),
            value: Some(ValueHolder {
                value: Value::String(value),
            }),
        }
    }

    pub fn new_u64(value: u64) -> Self {
        Var {
            def: VarDef::anonymous_string(),
            value: Some(ValueHolder {
                value: Value::U64(value),
            }),
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        self.value.as_ref().and_then(|var| {
            let Value::String(s) = &var.value else {
                return None;
            };

            Some(s.deref())
        })
    }

    pub fn as_rc_str(&self) -> Option<Rc<str>> {
        self.value.as_ref().and_then(|var| {
            let Value::String(s) = &var.value else {
                return None;
            };

            Some(s.clone())
        })
    }

    pub fn as_map(&self) -> Option<&RefCell<HashMap<PrimitiveValue, Value>>> {
        self.value.as_ref().and_then(|var| {
            let Value::Map(s) = &var.value else {
                return None;
            };

            Some(s.deref())
        })
    }

    pub fn as_list(&self) -> Option<&RefCell<Vec<Value>>> {
        self.value.as_ref().and_then(|var| {
            let Value::List(s) = &var.value else {
                return None;
            };

            Some(s.deref())
        })
    }
}