use alloc::vec::Vec;
use core::marker::PhantomData;
use core::mem;
use core::mem::ManuallyDrop;

use crate::temporary_collection::{TemporaryRefCollection, TemporaryRefCollectionType};

pub struct RefVec<T: ?Sized> {
    phantom: PhantomData<T>,
}

impl<T: ?Sized + 'static> TemporaryRefCollectionType for RefVec<T> {
    type Wrap<'a> = Vec<&'a T>;
    type Mut<'a> = Vec<&'a T>;
    type Raw = (*mut *const T, usize);

    fn new_raw() -> Self::Raw {
        let vec = Vec::<&'static T>::new();
        let mut vec = ManuallyDrop::new(vec);

        let capacity = vec.capacity();
        (vec.as_mut_ptr() as *mut *const T, capacity)
    }

    fn drop_raw(raw: &mut Self::Raw) {
        let (ptr, capacity) = raw;
        let ptr = *ptr as *mut &T;

        let vec = unsafe { Vec::from_raw_parts(ptr, 0, *capacity) };
        drop(vec);
    }

    fn capacity(raw: &Self::Raw) -> usize {
        let (_, capacity) = raw;
        *capacity
    }

    fn new_instance<'a, 'b>(raw: &'a Self::Raw) -> Self::Wrap<'b> {
        let (ptr, capacity) = raw;
        let ptr = *ptr as *mut &T;
        let vec = unsafe { Vec::from_raw_parts(ptr, 0, *capacity) };
        vec
    }

    fn instance_as_mut<'a>(instance: &'a mut Self::Wrap<'a>) -> &'a mut Self::Mut<'a> {
        instance
    }

    fn drop_instance(instance: &mut Self::Wrap<'_>, raw: &mut Self::Raw) {
        let mut vec = Vec::with_capacity(0);
        mem::swap(instance, &mut vec);

        let mut vec = ManuallyDrop::new(vec);

        let capacity = vec.capacity();
        *raw = (vec.as_mut_ptr() as *mut *const T, capacity)
    }
}

pub type TemporaryRefVec<T> = TemporaryRefCollection<RefVec<T>>;
