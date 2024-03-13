use bytemuck::{Pod, Zeroable};
use core::marker::PhantomData;

pub mod serializer;
pub mod slice;

pub trait AnnotatedTrait {
    type Value;
    type Annotation;

    fn value(&self) -> &Self::Value;
    fn value_mut(&mut self) -> &mut Self::Value;
}

#[repr(transparent)]
pub struct Annotated<Value, Annotation> {
    pub value: Value,
    phantom: PhantomData<Annotation>,
}

impl<Value: Copy, Annotation> Copy for Annotated<Value, Annotation> {}
unsafe impl<Value: Zeroable, Annotation> Zeroable for Annotated<Value, Annotation> {}
unsafe impl<Value: Pod, Annotation: 'static> Pod for Annotated<Value, Annotation> {}

impl<Value, Annotation> AnnotatedTrait for Annotated<Value, Annotation> {
    type Value = Value;
    type Annotation = Annotation;

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn value_mut(&mut self) -> &mut Self::Value {
        &mut self.value
    }
}

#[repr(transparent)]
pub struct FlatbufferAnnotated<Value, Annotation> {
    pub value: Value,
    phantom: PhantomData<Annotation>,
}

impl<Value, Annotation> AnnotatedTrait for FlatbufferAnnotated<Value, Annotation> {
    type Value = Value;
    type Annotation = Annotation;

    fn value(&self) -> &Self::Value {
        &self.value
    }

    fn value_mut(&mut self) -> &mut Self::Value {
        &mut self.value
    }
}

#[repr(transparent)]
pub struct InputOutputAnnotated<Value, InputAnnotation, OutputAnnotation> {
    pub value: Value,
    phantom: PhantomData<(InputAnnotation, OutputAnnotation)>,
}

impl<A, B, C> From<A> for InputOutputAnnotated<A, B, C> {
    fn from(value: A) -> Self {
        Self {
            value,
            phantom: PhantomData::default(),
        }
    }
}

macro_rules! impl_simple_annotation {
    ($type:ident) => {
        impl<A, B> From<A> for $type<A, B> {
            fn from(value: A) -> Self {
                Self {
                    value,
                    phantom: PhantomData::default(),
                }
            }
        }

        impl<A: Clone, B> Clone for $type<A, B> {
            fn clone(&self) -> Self {
                Self {
                    value: self.value.clone(),
                    phantom: PhantomData::default(),
                }
            }
        }
    };
}

impl_simple_annotation!(Annotated);
impl_simple_annotation!(FlatbufferAnnotated);
