use crate::http::errors::HttpError;
use crate::http::request::Request;

use regex::Regex;
use regex::RegexBuilder;

pub trait ContentTypeValidation: Request {
    fn allow_only_utf8_json_by_default(&self) -> Result<(), HttpError>;
}

impl<T: Request> ContentTypeValidation for T {
    fn allow_only_utf8_json_by_default(&self) -> Result<(), HttpError> {
        let content_type = self.get_header("Content-Type");

        let Some(content_type) = content_type else {
            return Ok(());
        };

        if is_utf8_json_content_type(content_type) {
            return Ok(());
        }

        Err(HttpError::ContentTypeUnsupported(
            "supported Content-Types: application/json, supported charsets: utf-8",
        ))
    }
}

fn is_utf8_json_content_type(value: &str) -> bool {
    lazy_static::lazy_static! {
        static ref RE: Regex =
            RegexBuilder::new("^application/json(;\\s*charset=utf-8)?$")
                .case_insensitive(true)
                .build()
                .unwrap();
    }

    RE.is_match(value)
}

#[cfg(test)]
mod tests {
    use crate::http::validation::content_type::is_utf8_json_content_type;

    #[test]
    fn test_utf8_json_validation() {
        assert!(is_utf8_json_content_type("application/json"));
        assert!(is_utf8_json_content_type("application/json; charset=utf-8"));
        assert!(is_utf8_json_content_type("application/json;charset=utf-8"));

        assert!(!is_utf8_json_content_type("text/plain"));
        assert!(!is_utf8_json_content_type(
            "application/json; charset=abracadabra"
        ));
    }
}
