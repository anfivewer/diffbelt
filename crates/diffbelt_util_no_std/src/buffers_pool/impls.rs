use crate::buffers_pool::PooledBuffer;
use alloc::vec::Vec;

impl<T> PooledBuffer for Vec<T> {
    type Item = Self;

    fn new() -> Self::Item {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self::Item {
        Vec::with_capacity(capacity)
    }

    fn ensure_capacity(buffer: &mut Self::Item, capacity: usize) {
        if capacity <= buffer.capacity() {
            return;
        }

        buffer.reserve(capacity - buffer.len());
    }

    fn clear(buffer: &mut Self::Item) {
        buffer.clear();
    }
}
