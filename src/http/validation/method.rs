use crate::http::errors::HttpError;
use crate::http::request::Request;

pub trait MethodsValidation: Request {
    fn allow_only_methods(&self, methods: &[&str]) -> Result<(), HttpError>;
}

impl<T: Request> MethodsValidation for T {
    fn allow_only_methods(&self, methods: &[&str]) -> Result<(), HttpError> {
        let method = self.method();

        let is_ok = methods.contains(&method);
        if is_ok {
            return Ok(());
        }

        Err(HttpError::MethodNotAllowed)
    }
}
