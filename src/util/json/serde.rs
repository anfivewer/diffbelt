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
    use serde::{Deserialize, Serialize};
    use serde_with::skip_serializing_none;

    #[derive(Deserialize, Debug)]
    struct WithStrictNull {
        #[serde(deserialize_with = "deserialize_strict_null")]
        value: Option<String>,
    }

    #[test]
    fn test_strict_null() {
        let result: Result<WithStrictNull, _> = serde_json::from_str(r#"{"value":"some str"}"#);
        assert_eq!(result.unwrap().value, Some("some str".to_string()));

        let result: Result<WithStrictNull, _> = serde_json::from_str(r#"{"value":null}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, None);

        let result: Result<WithStrictNull, _> = serde_json::from_str(r"{}");
        assert!(result.is_err());
    }

    #[derive(Deserialize, Debug)]
    struct NonStrictNull {
        value: Option<String>,
    }

    #[test]
    fn test_non_strict_null() {
        let result: Result<NonStrictNull, _> = serde_json::from_str(r#"{"value":"some str"}"#);
        assert_eq!(result.unwrap().value, Some("some str".to_string()));

        let result: Result<NonStrictNull, _> = serde_json::from_str(r#"{"value":null}"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().value, None);

        let result: Result<NonStrictNull, _> = serde_json::from_str(r"{}");
        assert_eq!(result.unwrap().value, None);
    }

    #[skip_serializing_none]
    #[derive(Serialize, Debug)]
    struct WithCustomNull {
        value: Option<Option<String>>,
    }

    #[test]
    fn test_custom_null() {
        let data = WithCustomNull { value: None };
        assert_eq!(serde_json::to_string(&data).unwrap(), r#"{}"#);

        let data = WithCustomNull { value: Some(None) };
        assert_eq!(serde_json::to_string(&data).unwrap(), r#"{"value":null}"#);

        let data = WithCustomNull {
            value: Some(Some("test".to_string())),
        };
        assert_eq!(serde_json::to_string(&data).unwrap(), r#"{"value":"test"}"#);
    }
}
