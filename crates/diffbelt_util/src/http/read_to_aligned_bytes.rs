use crate::http::error::BodyReadError;
use diffbelt_aligned_bytes::OwnedAlignedBytes;
use futures::future::BoxFuture;
use hyper::body::{Buf, HttpBody};
use hyper::Body;

pub type IntoAlignedBytesReturn<const ALIGN: usize> =
    BoxFuture<'static, Result<OwnedAlignedBytes<ALIGN>, BodyReadError>>;

pub fn into_aligned_bytes<const ALIGN: usize>(
    mut body: Body,
    max_size: usize,
) -> IntoAlignedBytesReturn<ALIGN> {
    Box::pin(async move {
        let mut full = Vec::with_capacity(ALIGN);
        // Waste ALIGN bytes to be able to move bytes backwards without additional allocation
        full.extend_from_slice(&[0; ALIGN]);

        let mut total_size = 0;

        while let Some(buf) = body.data().await {
            let buf = buf.or(Err(BodyReadError::IO))?;
            if !buf.has_remaining() {
                break;
            }

            total_size += buf.len();
            if total_size > max_size {
                return Err(BodyReadError::SizeLimit);
            }

            full.extend_from_slice(buf.as_ref());
        }

        let aligned = OwnedAlignedBytes::new(full, ALIGN, total_size)
            .map_err(|err| BodyReadError::Align(err))?;

        Ok(aligned)
    })
}
