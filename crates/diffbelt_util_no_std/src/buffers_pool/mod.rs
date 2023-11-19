pub mod impls;

use alloc::vec::Vec;

pub trait PooledBuffer {
    type Item;

    fn new() -> Self::Item;
}

pub struct BuffersPool<B: PooledBuffer> {
    pool: Vec<B::Item>,
}

impl<Buffer: PooledBuffer> BuffersPool<Buffer> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            pool: Vec::with_capacity(capacity),
        }
    }

    pub fn push(&mut self, buffer: Buffer::Item) {
        self.pool.push(buffer);
    }

    pub fn take(&mut self) -> Buffer::Item {
        self.pool.pop().unwrap_or_else(|| Buffer::new())
    }

    pub fn provide_as_option<R, E, F: FnOnce(&mut Option<Buffer::Item>) -> Result<R, E>>(
        &mut self,
        fun: F,
    ) -> Result<R, E> {
        let buffer = self.take();
        let mut buffer = Some(buffer);

        let result = fun(&mut buffer);

        if let Some(buffer) = buffer {
            self.pool.push(buffer);
        }

        result
    }
}

#[macro_export]
macro_rules! try_or_return_with_buffer_back {
    ($expr:expr, $buffer_opt:ident, $into_buffer:expr) => {
        match $expr {
            Ok(ok) => ok,
            Err(err) => {
                _ = $buffer_opt.insert($into_buffer);
                return Err(err);
            }
        }
    };
}
