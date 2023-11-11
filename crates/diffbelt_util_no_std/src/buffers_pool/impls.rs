use alloc::vec::Vec;
use crate::buffers_pool::PooledBuffer;

impl <T> PooledBuffer for Vec<T> {
    type Item = Self;

    fn new() -> Self::Item {
        Vec::new()
    }
}
