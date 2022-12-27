use crate::http::request::FullBody;
use futures::future::BoxFuture;

pub enum RequestReadError {
    IO,
    SizeLimit,
}

pub type IntoFullBodyAsReadReturn = BoxFuture<'static, Result<FullBody, RequestReadError>>;

pub trait Request {
    fn method(&self) -> &str;
    fn get_header(&self, name: &str) -> Option<&str>;
    fn reduce_multi_header<R, F: FnMut(R, &str) -> R>(
        &self,
        name: &str,
        reducer: F,
        initial: R,
    ) -> R;
    fn into_full_body_as_read(self, max_size: usize) -> IntoFullBodyAsReadReturn;
}
