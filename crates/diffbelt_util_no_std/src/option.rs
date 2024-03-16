use core::future::Future;

pub trait AsyncOptionUtil<A> {
    fn map_async<T, Fut: Future<Output = T>, F: FnOnce(A) -> Fut>(
        self,
        fun: F,
    ) -> impl Future<Output = Option<T>>;
}

impl<A> AsyncOptionUtil<A> for Option<A> {
    async fn map_async<T, Fut: Future<Output = T>, F: FnOnce(A) -> Fut>(self, fun: F) -> Option<T> {
        let Some(value) = self else {
            return None;
        };

        let value = fun(value).await;

        Some(value)
    }
}
