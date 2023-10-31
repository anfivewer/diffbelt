#[macro_export]
macro_rules! impl_from_either {
    ($T:ident) => {
        impl<A: Into<$T>, B: Into<$T>> From<::either::Either<A, B>> for $T {
            fn from(value: ::either::Either<A, B>) -> Self {
                match value {
                    ::either::Either::Left(value) => value.into(),
                    ::either::Either::Right(err) => err.into(),
                }
            }
        }
    };
}
