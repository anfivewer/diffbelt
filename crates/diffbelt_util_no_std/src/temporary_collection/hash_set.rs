use core::marker::PhantomData;
use core::mem;

use hashbrown::HashSet;

use crate::temporary_collection::{TemporaryRefCollection, TemporaryRefCollectionType};

pub struct RefHashSet<T: ?Sized> {
    phantom: PhantomData<T>,
}

impl<T: ?Sized + 'static> TemporaryRefCollectionType for RefHashSet<T> {
    type Wrap<'a> = *mut HashSet<&'a T>;
    type Mut<'a> = HashSet<&'a T>;
    type Raw = HashSet<&'static T>;

    fn new_raw() -> Self::Raw {
        HashSet::<&'static T>::new()
    }

    fn drop_raw(_raw: &mut Self::Raw) {}

    fn capacity(raw: &Self::Raw) -> usize {
        raw.capacity()
    }

    #[allow(mutable_transmutes)]
    fn new_instance<'a, 'b>(raw: &'a Self::Raw) -> Self::Wrap<'b> {
        raw as *const HashSet<&'static T> as *mut HashSet<&'b T>
    }

    fn instance_as_mut<'a>(instance: &'a mut Self::Wrap<'a>) -> &'a mut Self::Mut<'a> {
        let ptr = *instance;
        unsafe { &mut *ptr }
    }

    fn drop_instance(_instance: &mut Self::Wrap<'_>, _raw: &mut Self::Raw) {}
}

pub type TemporaryRefHashSet<T> = TemporaryRefCollection<RefHashSet<T>>;
