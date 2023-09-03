use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::{Var, VarDef};

impl<'a> FunctionInitState<'a> {
    pub fn temp_var(&mut self, def: VarDef, cleanups: &mut Vec<Statement>) -> VarPointer {
        let var = Var { def, value: None };

        let index = self.free_temp_var_indices.pop();

        if let Some(index) = index {
            self.vars[index] = var;

            return VarPointer::VarIndex(index);
        }

        let index = self.vars.len();
        cleanups.push(Statement::FreeTempVar(index));

        self.vars.push(var);

        VarPointer::VarIndex(index)
    }

    pub fn free_temp_var(&mut self, index: usize) {
        self.free_temp_var_indices.push(index);
    }
}
