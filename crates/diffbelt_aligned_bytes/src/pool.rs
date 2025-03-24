use alloc::sync::Arc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::mem;

pub struct BuffersForRealignPool {
    inner: RefCell<Inner>,
}

struct Inner {
    buffers: Vec<Vec<u8>>,
}

pub struct BufferHolder<'a> {
    parent: &'a BuffersForRealignPool,
    buffer: Vec<u8>,
}

impl<'a> BufferHolder<'a> {
    pub fn as_mut(&mut self) -> &mut Vec<u8> {
        &mut self.buffer
    }
}

impl<'a> Drop for BufferHolder<'a> {
    fn drop(&mut self) {
        let mut inner = self.parent.inner.borrow_mut();
        let buffer = mem::replace(&mut self.buffer, Vec::with_capacity(0));
        inner.buffers.push(buffer);
    }
}

impl BuffersForRealignPool {
    pub const fn new() -> Self {
        Self {
            inner: RefCell::new(Inner {
                buffers: Vec::new(),
            }),
        }
    }

    pub fn init(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.buffers.reserve(4);
    }

    pub fn take(&self) -> BufferHolder<'_> {
        let mut inner = self.inner.borrow_mut();
        let buffer = inner.buffers.pop().unwrap_or_default();

        BufferHolder {
            parent: self,
            buffer,
        }
    }
}
