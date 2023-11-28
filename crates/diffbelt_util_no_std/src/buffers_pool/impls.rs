use alloc::vec::Vec;
use crate::buffers_pool::PooledBuffer;

impl <T> PooledBuffer for Vec<T> {
    type Item = Self;

    fn new() -> Self::Item {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self::Item {
        Vec::with_capacity(capacity)
    }

    fn ensure_capacity(mut buffer: &mut Self::Item, capacity: usize) {
        if capacity <= buffer.capacity() {
            return;
        }

        buffer.reserve(capacity - buffer.capacity())
    }

    fn clear(buffer: &mut Self::Item) {
        buffer.clear();
    }
}
