pub use either::Either;

#[macro_export]
macro_rules! impl_from_either {
    ($T:ident) => {
        impl<A: Into<$T>, B: Into<$T>> From<::diffbelt_util_no_std::from_either::Either<A, B>> for $T {
            fn from(value: ::diffbelt_util_no_std::from_either::Either<A, B>) -> Self {
                match value {
                    ::diffbelt_util_no_std::from_either::Either::Left(value) => value.into(),
                    ::diffbelt_util_no_std::from_either::Either::Right(err) => err.into(),
                }
            }
        }
    };
}
