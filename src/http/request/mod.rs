use crate::http::routing::{IntoFullBodyAsReadReturn, Request as RoutingRequest, RequestReadError};
use futures::future::BoxFuture;
use hyper::body::{Buf, Bytes, HttpBody};
use hyper::{Body, Request};
use std::collections::VecDeque;

mod full_read;

pub struct HyperRequest {
    inner: Request<Body>,
}

pub struct HyperBody {
    inner: Body,
}

pub struct FullBody {
    bufs: VecDeque<Bytes>,
    offset: usize,
}

impl HyperRequest {
    pub fn from(request: Request<Body>) -> Self {
        Self { inner: request }
    }
}

impl RoutingRequest for HyperRequest {
    fn method(&self) -> &str {
        self.inner.method().as_str()
    }

    fn get_header(&self, name: &str) -> Option<&str> {
        let headers = self.inner.headers();

        let value = headers.get(name);
        let value = match value {
            Some(x) => x,
            None => {
                return None;
            }
        };

        let value = match value.to_str() {
            Ok(x) => x,
            Err(_) => {
                return None;
            }
        };

        Some(value)
    }

    fn reduce_multi_header<R, F: FnMut(R, &str) -> R>(
        &self,
        name: &str,
        mut reducer: F,
        initial: R,
    ) -> R {
        let headers = self.inner.headers();

        let value = headers.get(name);

        if let Some(value) = value {
            if let Ok(value) = value.to_str() {
                return reducer(initial, value);
            }
        }
        // headers.get_all(name)

        initial
    }

    fn into_full_body_as_read(self, max_size: usize) -> IntoFullBodyAsReadReturn {
        Box::pin(async move {
            let req = self.inner;
            let mut body = req.into_body();

            let mut bufs = VecDeque::new();
            let mut total_size = 0;

            while let Some(buf) = body.data().await {
                let buf = buf.or(Err(RequestReadError::IO))?;
                if !buf.has_remaining() {
                    break;
                }

                total_size += buf.len();
                if total_size > max_size {
                    return Err(RequestReadError::SizeLimit);
                }

                bufs.push_back(buf);
            }

            let full = FullBody { bufs, offset: 0 };

            Ok(Box::new(full) as Box<dyn std::io::Read>)
        })
    }
}

// async fn read_body_capped(body: Body) -> Result<impl Reader, T::Error>
//     where
//         T: HttpBody,
// {
//     let mut bufs = BufList::new();
//
//     futures_util::pin_mut!(body);
//     while let Some(buf) = body.data().await {
//         let buf = buf?;
//         if buf.has_remaining() {
//             bufs.push(buf);
//         }
//     }
//
//     Ok(bufs)
// }
