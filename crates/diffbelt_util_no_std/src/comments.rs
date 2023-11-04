use core::marker::PhantomData;
use core::ops::Deref;

#[repr(transparent)]
pub struct Annotated<Value, Annotation> {
    pub value: Value,
    phantom: PhantomData<Annotation>,
}

impl<A, B> From<A> for Annotated<A, B> {
    fn from(value: A) -> Self {
        Self {
            value,
            phantom: PhantomData::default(),
        }
    }
}

impl<A, B> Deref for Annotated<A, B> {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
