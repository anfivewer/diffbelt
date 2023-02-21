use std::str::from_utf8;
use std::string::ToString;

#[derive(Copy, Clone)]
pub enum StrSerializationType {
    Utf8,
    Base64,
}

impl StrSerializationType {
    pub fn from_option_string(t: Option<String>) -> Result<StrSerializationType, ()> {
        let Some(t) = t else { return Ok(StrSerializationType::Utf8); };

        Self::from_str(&t)
    }

    pub fn from_opt_str<T: AsRef<str>>(str_like: Option<T>) -> Result<Self, ()> {
        let Some(t) = str_like else { return Ok(StrSerializationType::Utf8); };

        Self::from_str(t.as_ref())
    }

    pub fn from_str(t: &str) -> Result<StrSerializationType, ()> {
        let t = match t {
            "utf8" => StrSerializationType::Utf8,
            "base64" => StrSerializationType::Base64,
            _ => {
                return Err(());
            }
        };

        Ok(t)
    }

    pub fn to_optional_string(&self) -> Option<String> {
        match self {
            StrSerializationType::Utf8 => None,
            StrSerializationType::Base64 => Some("base64".to_string()),
        }
    }

    pub fn serialize_with_priority(&self, bytes: &[u8]) -> (String, StrSerializationType) {
        match self {
            StrSerializationType::Utf8 => {
                // Check that all characters are visible
                let is_serializable = is_serializable_as_utf8(bytes);
                if !is_serializable {
                    return (
                        StrSerializationType::serialize_to_base64(bytes),
                        StrSerializationType::Base64,
                    );
                }

                // if it's valid utf8, use it
                let result = from_utf8(bytes);
                match result {
                    Ok(s) => (s.to_string(), StrSerializationType::Utf8),
                    Err(_) => (
                        StrSerializationType::serialize_to_base64(bytes),
                        StrSerializationType::Base64,
                    ),
                }
            }
            StrSerializationType::Base64 => (
                StrSerializationType::serialize_to_base64(bytes),
                StrSerializationType::Base64,
            ),
        }
    }

    pub fn serialize_to_base64(bytes: &[u8]) -> String {
        base64::encode(bytes)
    }

    pub fn deserialize<T: AsRef<str>>(&self, s: T) -> Result<Box<[u8]>, ()> {
        match self {
            StrSerializationType::Utf8 => {
                Ok(s.as_ref().to_string().into_bytes().into_boxed_slice())
            }
            StrSerializationType::Base64 => {
                Ok(base64::decode(s.as_ref()).or(Err(()))?.into_boxed_slice())
            }
        }
    }
}

fn is_serializable_as_utf8(bytes: &[u8]) -> bool {
    for b in bytes {
        let is_visible = 32 <= *b && *b <= 126;

        if !is_visible {
            return false;
        }
    }

    true
}
