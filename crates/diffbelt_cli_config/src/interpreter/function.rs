use crate::code::Code;
use crate::interpreter::error::InterpreterError;
use crate::interpreter::expression::VarPointer;
use crate::interpreter::statement::Statement;
use crate::interpreter::var::{Var, VarDef};
use crate::CliConfig;

use indexmap::IndexMap;
use std::collections::HashMap;

use std::rc::Rc;

#[derive(Debug, Clone)]
pub struct Function {
    pub input_vars_def: IndexMap<Rc<str>, VarDef>,
    pub vars: Vec<Var>,
    pub statements: Vec<Statement>,
}

pub struct FunctionInitState<'a> {
    pub config: &'a CliConfig,
    pub is_const_input_vars: bool,
    pub input_vars: IndexMap<Rc<str>, VarDef>,
    pub named_vars: HashMap<Rc<str>, Vec<VarPointer>>,
    pub vars: Vec<Var>,
    pub free_temp_var_indices: Vec<usize>,
    pub statements: Vec<Statement>,
}

impl Function {
    pub fn from_code(
        config: &CliConfig,
        code: &Code,
        input_vars: Option<IndexMap<Rc<str>, VarDef>>,
    ) -> Result<Self, InterpreterError> {
        let (is_const_input_vars, input_vars) = match input_vars {
            Some(input_vars) => (true, input_vars),
            None => (false, Default::default()),
        };

        let mut named_vars = HashMap::new();

        let vars = {
            let mut vars = Vec::with_capacity(input_vars.len());

            for (index, (name, var_def)) in input_vars.iter().enumerate() {
                vars.push(Var {
                    def: var_def.clone(),
                    value: None,
                });

                named_vars.insert(name.clone(), vec![VarPointer::VarIndex(index)]);
            }

            vars
        };

        let mut state = FunctionInitState {
            config,
            is_const_input_vars,
            input_vars,
            named_vars,
            vars,
            free_temp_var_indices: Vec::new(),
            statements: Vec::new(),
        };

        state.process_code(code)?;

        let FunctionInitState {
            input_vars,
            vars,
            statements,
            ..
        } = state;

        Ok(Self {
            input_vars_def: input_vars,
            vars,
            statements,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::interpreter::function::Function;
    use crate::interpreter::value::{Value, ValueHolder};
    use crate::interpreter::var::{Var, VarDef};
    use crate::CliConfig;
    use diffbelt_yaml::parse_yaml;
    use std::rc::Rc;

    #[test]
    fn create_function_test() {
        let config_str = include_str!("../../../../examples/cli-config.yaml");

        let docs = parse_yaml(config_str).expect("parsing");
        let doc = &docs[0];
        let config = CliConfig::from_yaml(doc).expect("reading");

        let code = &config.transforms[0];
        let code = code
            .map_filter
            .as_ref()
            .expect("first transform is not mapFilter");

        let input_vars = [(Rc::from("source"), VarDef::anonymous_string())]
            .into_iter()
            .collect();

        let function = Function::from_code(&config, code, Some(input_vars)).expect("function");

        let input_vars = vec![(
            Rc::from("source"),
            Var {
                def: VarDef::anonymous_string(),
                value: Some(ValueHolder { value: Value::String(Rc::from("S 2023-02-20T21:42:48.822Z.000 worker258688:middlewares handleFull updateType:edited_message ms:27")) }),
            },
        )]
            .into_iter().collect();

        function.call(input_vars).expect("function execution");
    }
}
