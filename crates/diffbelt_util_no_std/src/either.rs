use either::Either;

#[inline(always)]
pub fn left_if_some<T>(option: Option<T>) -> Either<T, Option<T>> {
    let Some(value) = option else {
        return Either::Right(None);
    };

    Either::Left(value)
}
