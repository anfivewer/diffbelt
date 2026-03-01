use alloc::boxed::Box;
use diffbelt_wasm_binding::debug_print;
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};
use diffbelt_aligned_bytes::pool::{BufferHolder, BuffersForRealignPool};

pub static BUFFERS_FOR_REALIGN: AtomicPtr<BuffersForRealignPool> = AtomicPtr::new(ptr::null_mut());

pub fn take_buffer_for_realign() -> BufferHolder<'static> {
    debug_print("1");
    let pool_ptr = BUFFERS_FOR_REALIGN.load(Ordering::Relaxed);
    if !pool_ptr.is_null() {
        // SAFETY: we are never dealloc it and we never run in multiple threads
        let pool = unsafe { &*pool_ptr };
        let holder = pool.take();

        return holder;
    }

    let mut pool = Box::new(BuffersForRealignPool::new());
    let new_pool_ptr = pool.as_mut() as *mut BuffersForRealignPool;

    let pool_ptr = match BUFFERS_FOR_REALIGN.compare_exchange(
        pool_ptr,
        new_pool_ptr,
        Ordering::Relaxed,
        Ordering::Relaxed,
    ) {
        Ok(pool_ptr) => {
            Box::leak(pool);
            pool_ptr
        }
        Err(pool_ptr) => {
            drop(pool);
            assert!(!pool_ptr.is_null());
            pool_ptr
        }
    };

    // SAFETY: we are leaked existing Box or checked for non-null
    let pool = unsafe { &*pool_ptr };

    pool.take()
}
