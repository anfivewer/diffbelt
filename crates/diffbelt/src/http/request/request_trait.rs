use diffbelt_util::http::read_full_body::IntoFullBodyAsReadReturn;
use std::borrow::Cow;

pub trait Request {
    fn method(&self) -> &str;
    fn get_path(&self) -> &str;
    fn query_params(&self) -> Result<Vec<(Cow<str>, Cow<str>)>, ()>;
    fn get_header(&self, name: &str) -> Option<&str>;
    fn reduce_multi_header<R, F: FnMut(R, &str) -> R>(
        &self,
        name: &str,
        reducer: F,
        initial: R,
    ) -> R;
    fn into_full_body_as_read(self, max_size: usize) -> IntoFullBodyAsReadReturn;
}
