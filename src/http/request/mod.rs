use hyper::body::{Buf, Bytes, HttpBody};
use hyper::{Body, Request as HyperRequest};
pub use request_trait::*;
use std::borrow::Cow;
use std::collections::VecDeque;

mod full_read;
mod request_trait;

pub struct HyperRequestWrapped {
    inner: HyperRequest<Body>,
}

pub struct FullBody {
    bufs: VecDeque<Bytes>,
    offset: usize,
}

impl HyperRequestWrapped {
    pub fn from(request: HyperRequest<Body>) -> Self {
        Self { inner: request }
    }
}

impl Request for HyperRequestWrapped {
    fn method(&self) -> &str {
        self.inner.method().as_str()
    }

    fn get_path(&self) -> &str {
        self.inner.uri().path()
    }

    fn query_params(&self) -> Result<Vec<(Cow<str>, Cow<str>)>, ()> {
        let query = self.inner.uri().query();
        let Some(query) = query else {
            return Ok(Vec::with_capacity(0));
        };

        let params = querystring::querify(query);
        let mut result = Vec::with_capacity(params.len());

        for (key, value) in params {
            let key = urlencoding::decode(key).map_err(|_| ())?;
            let value = urlencoding::decode(value).map_err(|_| ())?;

            result.push((key, value));
        }

        Ok(result)
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

            Ok(full)
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
