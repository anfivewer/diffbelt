use crate::interpreter::function::FunctionInitState;
use crate::interpreter::var::{Var, VarDef};

impl<'a> FunctionInitState<'a> {
    pub fn temp_var(&mut self, def: VarDef) -> usize {
        let var = Var { def, value: None };

        let index = self.free_temp_var_indices.pop();

        if let Some(index) = index {
            self.vars[index] = var;

            return index;
        }

        let index = self.vars.len();

        self.vars.push(var);

        index
    }

    pub fn free_temp_var(&mut self, index: usize) {
        self.free_temp_var_indices.push(index);
    }
}
