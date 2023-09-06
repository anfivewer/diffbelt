use crate::interpreter::error::InterpreterError;
use crate::interpreter::function::FunctionInitState;
use crate::interpreter::statement::Statement;
use std::ops::Deref;
use std::rc::Rc;

pub struct Cleanups {
    pub runtime: Vec<Statement>,
    pub compile_time: Vec<CompileTimeCleanup>,
}

impl Cleanups {
    pub fn new() -> Self {
        Self {
            runtime: Vec::new(),
            compile_time: Vec::new(),
        }
    }
}

impl<'a> FunctionInitState<'a> {
    pub fn apply_cleanups(&mut self, cleanups: Cleanups) -> Result<(), InterpreterError> {
        let Cleanups {
            runtime,
            compile_time,
        } = cleanups;

        self.push_statements(runtime);

        for cleanup in compile_time {
            match cleanup {
                CompileTimeCleanup::FreeTempVar(index) => {
                    self.free_temp_var(index);
                }
                CompileTimeCleanup::DropNamedVar(name) => {
                    self.drop_named_var(name.deref())?;
                }
            }
        }

        Ok(())
    }
}

pub enum CompileTimeCleanup {
    FreeTempVar(usize),
    DropNamedVar(Rc<str>),
}
