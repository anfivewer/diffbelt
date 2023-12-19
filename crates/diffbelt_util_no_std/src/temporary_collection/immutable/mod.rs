use core::fmt::{Debug, Formatter};

pub mod hash_map;
pub mod hash_set;
pub mod vec;

pub trait TemporaryRefCollectionType {
    type Wrap<'a>;
    type Mut<'a>;
    type Raw;

    fn new_raw() -> Self::Raw;
    fn drop_raw(raw: &mut Self::Raw);
    fn capacity(raw: &Self::Raw) -> usize;
    fn new_instance<'a, 'b>(raw: &'a Self::Raw) -> Self::Wrap<'b>;
    fn instance_as_mut<'a>(instance: &'a mut Self::Wrap<'static>) -> &'a mut Self::Mut<'a>;
    fn drop_instance(instance: &mut Self::Wrap<'_>, raw: &mut Self::Raw);
}

pub struct TemporaryRefCollection<T: TemporaryRefCollectionType> {
    raw: T::Raw,
}

impl<T: TemporaryRefCollectionType> Debug for TemporaryRefCollection<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("TemporaryRefVec { capacity = ")?;
        T::capacity(&self.raw).fmt(f)?;
        f.write_str(" }")?;
        Ok(())
    }
}

pub struct TemporaryRefCollectionInstance<'parent, T: TemporaryRefCollectionType> {
    instance: T::Wrap<'static>,
    parent: &'parent mut TemporaryRefCollection<T>,
}

impl<'parent, T: TemporaryRefCollectionType> TemporaryRefCollectionInstance<'parent, T> {
    pub fn as_mut(&mut self) -> &mut T::Mut<'_> {
        T::instance_as_mut(&mut self.instance)
    }
}

impl<'instance, 'parent, T: TemporaryRefCollectionType> Drop
    for TemporaryRefCollectionInstance<'parent, T>
{
    fn drop(&mut self) {
        T::drop_instance(&mut self.instance, &mut self.parent.raw);
    }
}

impl<T: TemporaryRefCollectionType> TemporaryRefCollection<T> {
    pub fn new() -> Self {
        Self { raw: T::new_raw() }
    }

    pub fn temp(&mut self) -> TemporaryRefCollectionInstance<'_, T> {
        let instance = T::new_instance(&mut self.raw);
        TemporaryRefCollectionInstance {
            instance,
            parent: self,
        }
    }
}

impl<T: TemporaryRefCollectionType> Drop for TemporaryRefCollection<T> {
    fn drop(&mut self) {
        T::drop_raw(&mut self.raw);
    }
}
