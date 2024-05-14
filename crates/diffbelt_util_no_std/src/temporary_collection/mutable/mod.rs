use core::fmt::{Debug, Formatter};

pub mod vec;

pub trait TemporaryMutRefCollectionType {
    type Wrap<'a>;
    type Mut<'a>;
    type Raw;

    fn new_raw() -> Self::Raw;
    fn drop_raw(raw: &mut Self::Raw);
    fn capacity(raw: &Self::Raw) -> usize;
    fn new_instance<'a, 'b>(raw: &'a mut Self::Raw) -> Self::Wrap<'b>;
    fn instance_as_mut<'a>(instance: &'a mut Self::Wrap<'static>) -> &'a mut Self::Mut<'a>;
    fn drop_instance(instance: &mut Self::Wrap<'_>, raw: &mut Self::Raw);
}

pub struct TemporaryMutRefCollection<T: TemporaryMutRefCollectionType> {
    raw: T::Raw,
}

impl<T: TemporaryMutRefCollectionType> Debug for TemporaryMutRefCollection<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("TemporaryMutRefVec { capacity = ")?;
        T::capacity(&self.raw).fmt(f)?;
        f.write_str(" }")?;
        Ok(())
    }
}

pub struct TemporaryMutRefCollectionInstance<'parent, T: TemporaryMutRefCollectionType> {
    instance: T::Wrap<'static>,
    parent: &'parent mut TemporaryMutRefCollection<T>,
}

impl<'parent, T: TemporaryMutRefCollectionType> TemporaryMutRefCollectionInstance<'parent, T> {
    pub fn as_mut(&mut self) -> &mut T::Mut<'_> {
        T::instance_as_mut(&mut self.instance)
    }
}

impl<'instance, 'parent, T: TemporaryMutRefCollectionType> Drop
    for TemporaryMutRefCollectionInstance<'parent, T>
{
    fn drop(&mut self) {
        T::drop_instance(&mut self.instance, &mut self.parent.raw);
    }
}

impl<T: TemporaryMutRefCollectionType> TemporaryMutRefCollection<T> {
    pub fn new() -> Self {
        Self { raw: T::new_raw() }
    }

    pub fn temp(&mut self) -> TemporaryMutRefCollectionInstance<'_, T> {
        let instance = T::new_instance(&mut self.raw);
        TemporaryMutRefCollectionInstance {
            instance,
            parent: self,
        }
    }
}

impl<T: TemporaryMutRefCollectionType> Drop for TemporaryMutRefCollection<T> {
    fn drop(&mut self) {
        T::drop_raw(&mut self.raw);
    }
}
