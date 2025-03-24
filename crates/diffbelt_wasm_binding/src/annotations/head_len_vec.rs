use core::marker::PhantomData;
use diffbelt_protos::FlatbuffersGenericType;

pub struct FlatbuffersHeadLenVecAnnotation<T: FlatbuffersGenericType> {
    phantom: PhantomData<T>,
}
