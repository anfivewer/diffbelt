use serde::{Deserialize, Deserializer};

pub fn deserialize_strict_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    T: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use crate::util::json::serde::deserialize_strict_null;
    use serde::Deserialize;

    #[derive(Deserialize, Debug)]
    struct TestStruct {
        #[serde(deserialize_with = "deserialize_strict_null")]
        value: Option<String>,
    }

    #[test]
    fn test_deserialize() {
        let result: Result<TestStruct, _> = serde_json::from_str(r#"{"value":"some str"}"#);
        assert_eq!(result.unwrap().value, Some("some str".to_string()));

        let result: Result<TestStruct, _> = serde_json::from_str(r#"{"value":null}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, None);

        let result: Result<TestStruct, _> = serde_json::from_str(r"{}");
        assert!(result.is_err());
    }
}
