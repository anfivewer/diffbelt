use core::marker::PhantomData;
use core::mem;

use hashbrown::HashMap;

use crate::temporary_collection::{TemporaryRefCollection, TemporaryRefCollectionType};

pub struct RefHashMap<K: ?Sized + 'static, V: ?Sized + 'static> {
    phantom: PhantomData<(&'static K, &'static V)>,
}

impl<K: ?Sized + 'static, V: ?Sized + 'static> TemporaryRefCollectionType for RefHashMap<K, V> {
    type Wrap<'a> = *mut HashMap<&'a K, &'a V>;
    type Mut<'a> = HashMap<&'a K, &'a V>;
    type Raw = HashMap<&'static K, &'static V>;

    fn new_raw() -> Self::Raw {
        HashMap::<&'static K, &'static V>::new()
    }

    fn drop_raw(_raw: &mut Self::Raw) {}

    fn capacity(raw: &Self::Raw) -> usize {
        raw.capacity()
    }

    #[allow(mutable_transmutes)]
    fn new_instance<'a, 'b>(raw: &'a Self::Raw) -> Self::Wrap<'b> {
        raw as *const HashMap<&'static K, &'static V> as *mut HashMap<&'b K, &'b V>
    }

    fn instance_as_mut<'a>(instance: &'a mut Self::Wrap<'static>) -> &'a mut Self::Mut<'a> {
        let ptr = *instance;
        unsafe { mem::transmute(&mut *ptr) }
    }

    fn drop_instance(_instance: &mut Self::Wrap<'_>, _raw: &mut Self::Raw) {}
}

pub type TemporaryRefHashMap<K, V> = TemporaryRefCollection<RefHashMap<K, V>>;
