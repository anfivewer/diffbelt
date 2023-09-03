use crate::interpreter::expression::VarPointer;

#[derive(Debug, Clone)]
pub struct ConcatStatement {
    pub parts: Vec<VarPointer>,
    pub destination: VarPointer,
}
