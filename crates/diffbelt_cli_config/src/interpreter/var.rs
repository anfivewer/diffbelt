use crate::interpreter::value::ValueHolder;

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
}

#[derive(Debug, Clone)]
pub struct Var {
    pub def: VarDef,
    pub value: Option<ValueHolder>,
}
