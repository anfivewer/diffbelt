use crate::interpreter::cleanups::{Cleanups, CompileTimeCleanup};
use crate::interpreter::expression::VarPointer;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::var::{Var, VarDef};

impl<'a> FunctionInitState<'a> {
    pub fn persist_var(&mut self, def: VarDef) -> VarPointer {
        let var = Var { def, value: None };

        let index = self.free_temp_var_indices.pop();

        if let Some(index) = index {
            self.vars[index] = var;

            return VarPointer::VarIndex(index);
        }

        let index = self.vars.len();

        self.vars.push(var);

        VarPointer::VarIndex(index)
    }

    pub fn temp_var(&mut self, def: VarDef, cleanups: &mut Cleanups) -> VarPointer {
        let ptr = self.persist_var(def);

        let VarPointer::VarIndex(index) = ptr else {
            panic!("persist_var returned not a pointer");
        };

        cleanups
            .compile_time
            .push(CompileTimeCleanup::FreeTempVar(index));

        ptr
    }

    pub fn free_temp_var(&mut self, index: usize) {
        self.free_temp_var_indices.push(index);
    }
}
