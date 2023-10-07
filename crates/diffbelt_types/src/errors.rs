#[derive(Debug)]
pub enum IntoBytesError<T> {
    UnknownEncoding(T),
    Base64(T),
}
