use crate::formats::ValueFormatError;
use crate::interpreter::value::{PrimitiveValue, Value, ValueHolder};
use crate::interpreter::var::{Var, VarDef};
use diffbelt_util::Wrap;
use serde_json::{Number, Value as JsonValue};
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Rc;

fn json_value_to_value(value: JsonValue) -> Result<Value, ValueFormatError> {
    let value = match value {
        JsonValue::Null => Value::None,
        JsonValue::Bool(value) => Value::Bool(value),
        JsonValue::Number(value) => {
            if let Some(value) = value.as_u64() {
                Value::U64(value)
            } else if let Some(value) = value.as_i64() {
                Value::I64(value)
            } else if let Some(value) = value.as_f64() {
                Value::F64(value)
            } else {
                return Err(ValueFormatError::JsonNotF64Impossible);
            }
        }
        JsonValue::String(s) => Value::String(Rc::from(s)),
        JsonValue::Array(arr) => {
            let mut values = Vec::with_capacity(arr.len());

            for value in arr {
                values.push(json_value_to_value(value)?);
            }

            Value::List(Wrap::wrap(values))
        }
        JsonValue::Object(obj) => {
            let mut map = HashMap::with_capacity(obj.len());

            for (key, value) in obj {
                map.insert(
                    PrimitiveValue::String(Rc::from(key)),
                    json_value_to_value(value)?,
                );
            }

            Value::Map(Wrap::wrap(map))
        }
    };

    Ok(value)
}

pub fn json_value_to_var(value: JsonValue) -> Result<Var, ValueFormatError> {
    let value = json_value_to_value(value)?;

    Ok(Var {
        def: VarDef::unknown(),
        value: Some(ValueHolder { value }),
    })
}

// TODO: implement Serialize for Value?
pub fn value_to_json_value(value: &Value) -> Result<JsonValue, ValueFormatError> {
    let mut visited_values_set = HashSet::new();

    fn inner(
        value: &Value,
        visited_values_set: &mut HashSet<*const Value>,
    ) -> Result<JsonValue, ValueFormatError> {
        let value = match value {
            Value::None => JsonValue::Null,
            Value::Bool(value) => JsonValue::Bool(*value),
            Value::String(s) => JsonValue::String(s.to_string()),
            value @ Value::Bytes(_) => {
                return Err(ValueFormatError::ValueCannotBeRepresentedAsUtf8(
                    value.clone(),
                ));
            }
            Value::U64(n) => JsonValue::Number(Number::from(*n)),
            Value::I64(n) => JsonValue::Number(Number::from(*n)),
            Value::F64(n) => {
                JsonValue::Number(Number::from_f64(*n).ok_or_else(|| ValueFormatError::BadF64(*n))?)
            }
            Value::List(list) => {
                let list = list.borrow();
                let list = list.deref();

                let mut values = Vec::with_capacity(list.len());

                for value in list {
                    let ptr = value as (*const Value);
                    if visited_values_set.contains(&ptr) {
                        return Err(ValueFormatError::CycleDetected);
                    }
                    visited_values_set.insert(ptr);

                    values.push(inner(value, visited_values_set)?);
                }

                JsonValue::Array(values)
            }
            Value::Map(map) => {
                let map = map.borrow();
                let map = map.deref();

                let mut values = serde_json::Map::with_capacity(map.len());

                for (key, value) in map {
                    let key = match key {
                        PrimitiveValue::String(s) => s.to_string(),
                    };
                    let value = inner(value, visited_values_set)?;

                    values.insert(key, value);
                }

                JsonValue::Object(values)
            }
        };

        Ok(value)
    }

    inner(value, &mut visited_values_set)
}
