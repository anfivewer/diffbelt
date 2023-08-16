use futures::future::BoxFuture;
use hyper::body::{Buf, Bytes, HttpBody};
use hyper::Body;
use std::collections::VecDeque;
use std::io::Read;

#[derive(Debug)]
pub enum BodyReadError {
    IO,
    SizeLimit,
}

pub struct FullBody {
    bufs: VecDeque<Bytes>,
    offset: usize,
}

pub type IntoFullBodyAsReadReturn = BoxFuture<'static, Result<FullBody, BodyReadError>>;

pub fn into_full_body_as_read(mut body: Body, max_size: usize) -> IntoFullBodyAsReadReturn {
    Box::pin(async move {
        let mut bufs = VecDeque::new();
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

            bufs.push_back(buf);
        }

        let full = FullBody { bufs, offset: 0 };

        Ok(full)
    })
}

// Implemented for Serde, but maybe we can do it with zero-copy somehow
impl Read for FullBody {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bufs = &mut self.bufs;
        let offset = &mut self.offset;

        let Some(mut bytes) = bufs.front() else { return Ok(0); };

        let mut bytes_written = 0;
        let mut result_offset = 0;
        let mut result_bytes_left = buf.len();

        'outer: while result_bytes_left > 0 {
            let size_left_in_current_buf = 'find_buf: loop {
                let size_left_in_current_buf = bytes.len() - *offset;
                if size_left_in_current_buf > 0 {
                    break 'find_buf size_left_in_current_buf;
                }

                bufs.pop_front();
                *offset = 0;

                let Some(front) = bufs.front() else { break 'outer; };
                bytes = front;
            };

            if result_bytes_left <= size_left_in_current_buf {
                buf[result_offset..(result_offset + result_bytes_left)]
                    .copy_from_slice(&bytes[*offset..(*offset + result_bytes_left)]);

                *offset += result_bytes_left;

                if result_bytes_left == size_left_in_current_buf {
                    bufs.pop_front();
                    *offset = 0;
                }

                bytes_written += result_bytes_left;

                break;
            }

            buf[result_offset..(result_offset + size_left_in_current_buf)]
                .copy_from_slice(&bytes[*offset..(*offset + size_left_in_current_buf)]);

            result_offset += size_left_in_current_buf;
            bytes_written += size_left_in_current_buf;
            result_bytes_left -= size_left_in_current_buf;

            bufs.pop_front();
            *offset = 0;

            let Some(front) = bufs.front() else { break; };
            bytes = front;
        }

        Ok(bytes_written)
    }
}
