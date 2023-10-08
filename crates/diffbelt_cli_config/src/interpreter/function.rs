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
    use crate::interpreter::value::{PrimitiveValue, Value, ValueHolder};
    use crate::interpreter::var::{Var, VarDef};
    use crate::CliConfig;
    use diffbelt_util::Wrap;
    use diffbelt_yaml::parse_yaml;
    use std::collections::HashMap;
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

        let input_vars = [
            (Rc::from("map_filter_key"), VarDef::anonymous_string()),
            (Rc::from("map_filter_new_value"), VarDef::anonymous_string()),
        ]
        .into_iter()
        .collect();

        let function = Function::from_code(&config, code, Some(input_vars)).expect("function");

        let key = Rc::<str>::from("S 2023-02-20T21:42:48.822Z.000 worker258688:middlewares handleFull updateType:edited_message ms:27 |some extra|another extra");

        let input_vars = vec![
            (
                Rc::from("map_filter_key"),
                Var {
                    def: VarDef::anonymous_string(),
                    value: Some(ValueHolder {
                        value: Value::String(key.clone()),
                    }),
                },
            ),
            (
                Rc::from("map_filter_new_value"),
                Var {
                    def: VarDef::anonymous_string(),
                    value: Some(ValueHolder {
                        value: Value::String(Rc::from("")),
                    }),
                },
            ),
        ]
        .into_iter()
        .collect();

        let actual_value = function.call(input_vars).expect("function execution");

        let expected_value = Value::Map(Wrap::wrap(HashMap::from([
            (
                PrimitiveValue::String(Rc::from("logLevel")),
                Value::String(Rc::from("S")),
            ),
            (
                PrimitiveValue::String(Rc::from("loggerKey")),
                Value::String(Rc::from("worker258688:middlewares")),
            ),
            (
                PrimitiveValue::String(Rc::from("timestampString")),
                Value::String(Rc::from("2023-02-20T21:42:48.822Z.000")),
            ),
            (
                PrimitiveValue::String(Rc::from("timestampMilliseconds")),
                Value::U64(1676929368822),
            ),
            (
                PrimitiveValue::String(Rc::from("timestampMicroseconds")),
                Value::U64(0),
            ),
            (
                PrimitiveValue::String(Rc::from("logKey")),
                Value::String(Rc::from("handleFull")),
            ),
            (
                PrimitiveValue::String(Rc::from("props")),
                Value::Map(Wrap::wrap(HashMap::from([
                    (
                        PrimitiveValue::String(Rc::from("updateType")),
                        Value::String(Rc::from("edited_message")),
                    ),
                    (
                        PrimitiveValue::String(Rc::from("ms")),
                        Value::String(Rc::from("27")),
                    ),
                ]))),
            ),
            (
                PrimitiveValue::String(Rc::from("extra")),
                Value::List(Wrap::wrap(vec![
                    Value::String(Rc::from("some extra")),
                    Value::String(Rc::from("another extra")),
                ])),
            ),
        ])));

        let expected_value = Value::Map(Wrap::wrap(HashMap::from([
            (PrimitiveValue::String(Rc::from("key")), Value::String(key)),
            (PrimitiveValue::String(Rc::from("value")), expected_value),
        ])));

        assert_eq!(actual_value, expected_value);
    }
}
