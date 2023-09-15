use crate::errors::ConfigPositionMark;
use crate::interpreter::expression::VarPointer;

#[derive(Debug, Clone)]
pub struct ParseDateToMsStatement {
    pub ptr: VarPointer,
    pub mark: ConfigPositionMark,
}
