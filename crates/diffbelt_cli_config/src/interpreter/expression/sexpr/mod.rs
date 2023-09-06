use crate::interpreter::error::InterpreterError;
use lexpr::parse::Brackets;

pub enum SExpr {
    //
}

impl SExpr {
    pub fn parse(s: &str) -> Result<Self, InterpreterError> {
        let as_invalid_expr_lexpr = |_| InterpreterError::InvalidExpression(s.to_string());
        let as_invalid_expr_opt = || InterpreterError::InvalidExpression(s.to_string());

        let expr = lexpr::from_str_custom(s, lexpr::parse::Options::new())
            .map_err(as_invalid_expr_lexpr)?;

        let expr = expr.as_cons().ok_or_else(as_invalid_expr_opt)?;
        let (name, params) = expr.as_pair();

        let name = name.as_symbol().ok_or_else(as_invalid_expr_opt)?;

        println!("parsed s-expr {:#?} {:#?}", name, params);

        todo!()
    }
}
