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

    pub fn anonymous_bytes() -> Self {
        Self::unknown()
    }

    pub fn anonymous_bool() -> Self {
        Self::unknown()
    }

    pub fn anonymous_string() -> Self {
        Self::unknown()
    }

    pub fn anonymous_u64() -> Self {
        Self::unknown()
    }
}

#[derive(Debug, Clone)]
pub struct Var {
    pub def: VarDef,
    pub value: Option<ValueHolder>,
}

impl Var {
    pub fn new_none() -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::None,
            }),
        }
    }

    pub fn new_bool(value: bool) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::Bool(value),
            }),
        }
    }

    pub fn new_bytes(value: Rc<[u8]>) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::Bytes(value),
            }),
        }
    }

    pub fn new_string(value: Rc<str>) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::String(value),
            }),
        }
    }

    pub fn new_u64(value: u64) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::U64(value),
            }),
        }
    }

    pub fn new_i64(value: i64) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::I64(value),
            }),
        }
    }

    pub fn new_f64(value: f64) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::F64(value),
            }),
        }
    }

    pub fn new_list(value: Rc<RefCell<Vec<Value>>>) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::List(value),
            }),
        }
    }

    pub fn new_map(value: Rc<RefCell<HashMap<PrimitiveValue, Value>>>) -> Self {
        Var {
            def: VarDef::unknown(),
            value: Some(ValueHolder {
                value: Value::Map(value),
            }),
        }
    }

    pub fn as_initialized_none(&self) -> Option<bool> {
        self.value.as_ref().map(|var| {
            let Value::None = &var.value else {
                return false;
            };

            true
        })
    }

    pub fn as_bool(&self) -> Option<bool> {
        self.value.as_ref().and_then(|var| {
            let Value::Bool(b) = &var.value else {
                return None;
            };

            Some(*b)
        })
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
