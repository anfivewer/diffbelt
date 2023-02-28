use std::sync::atomic::{AtomicPtr, Ordering};

pub struct AtomicCleanup<T> {
    atomic: AtomicPtr<T>,
}

impl<T> AtomicCleanup<T> {
    pub fn some(value: T) -> Self {
        let boxed = Box::new(value);
        let ptr = Box::leak(boxed);

        let atomic = AtomicPtr::new(ptr);

        AtomicCleanup { atomic }
    }

    pub fn take(&self) -> Option<Box<T>> {
        loop {
            let ptr = self.atomic.load(Ordering::Relaxed);

            if ptr.is_null() {
                return None;
            }

            let result = self.atomic.compare_exchange(
                ptr,
                std::ptr::null_mut(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            );

            if result.is_ok() {
                let boxed = unsafe { Box::from_raw(&mut *ptr) };
                return Some(boxed);
            }
        }
    }
}

impl<T> Drop for AtomicCleanup<T> {
    fn drop(&mut self) {
        self.take();
    }
}

mod tests {
    use crate::util::atomic_cleanup::AtomicCleanup;

    struct Dropable<'a> {
        is_dropped: &'a mut bool,
    }

    impl Drop for Dropable<'_> {
        fn drop(&mut self) {
            *self.is_dropped = true;
        }
    }

    #[test]
    fn test_atomic() {
        let mut is_dropped = false;

        {
            let atomic = AtomicCleanup::some(Dropable {
                is_dropped: &mut is_dropped,
            });

            let value = atomic.take();
            assert!(atomic.take().is_none());

            assert!(&value.is_some());

            let value = value.unwrap();
            assert_eq!(value.is_dropped, &mut false);
        }

        assert!(is_dropped);

        is_dropped = false;

        {
            AtomicCleanup::some(Dropable {
                is_dropped: &mut is_dropped,
            });
        }

        assert!(is_dropped);
    }
}
