use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;

#[derive(Debug, Clone)]
pub struct JumpIfStatement {
    pub condition: Condition,
    pub statement_index: usize,
}

#[derive(Debug, Clone)]
pub enum Condition {
    NonEmptyString(VarPointer),
}

impl<'a> FunctionInitState<'a> {
    pub fn jump_if(&mut self, condition: Condition) -> impl Fn(&mut FunctionInitState<'_>, usize) {
        let index = self.statements.len();

        self.statements.push(Statement::JumpIf(JumpIfStatement {
            condition,
            statement_index: usize::MAX,
        }));

        move |this, jump_index| {
            let statement = &mut this.statements[index];
            let Statement::JumpIf(statement) = statement else {
                panic!("statements list should be append-only");
            };

            statement.statement_index = jump_index;
        }
    }
}
