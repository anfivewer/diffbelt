#[deprecated(note = "use .transpose()")]
pub fn lift_result_from_option<R, E>(opt: Option<Result<R, E>>) -> Result<Option<R>, E> {
    match opt {
        Some(result) => match result {
            Ok(result) => Ok(Some(result)),
            Err(err) => Err(err),
        },
        None => Ok(None),
    }
}

#[deprecated(note = "use .flatten()")]
pub fn cut_layer<T>(opt: Option<Option<T>>) -> Option<T> {
    opt.and_then(|inner| inner)
}
