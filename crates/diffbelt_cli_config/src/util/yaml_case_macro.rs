#[macro_export]
macro_rules! decode_case {
    ( $yaml:expr, $enum_case:path ) => {
        if let Ok(value) = decode_yaml($yaml) {
            return Ok($enum_case(value));
        }
    };
}
