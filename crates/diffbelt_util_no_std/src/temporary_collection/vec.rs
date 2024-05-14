use alloc::vec::Vec;
use core::mem;

pub trait TempVecType {
    type Item<'a>;
}

pub struct TemporaryVec<T: TempVecType> {
    vec: Option<Vec<T::Item<'static>>>,
}

pub struct TemporaryVecInstance<'parent, 'a, T: TempVecType> {
    inner: Option<Vec<T::Item<'a>>>,
    parent: &'parent mut Option<Vec<T::Item<'static>>>,
}

impl<T: TempVecType> TemporaryVec<T> {
    pub fn new() -> Self {
        Self {
            vec: Some(Vec::new()),
        }
    }

    pub fn temp<'a, 'b: 'a>(&'b mut self) -> TemporaryVecInstance<'b, 'a, T> {
        let Some(vec) = self.vec.take() else {
            panic!("Multiple temp() calls");
        };

        // Convert 'static lifetime to 'a
        // `TemporaryVecInstance` will live while live self,
        // and on drop we will empty this vec and return it back
        let vec = unsafe { mem::transmute(vec) };

        TemporaryVecInstance {
            inner: Some(vec),
            parent: &mut self.vec,
        }
    }
}

impl<'parent, 'a, T: TempVecType> TemporaryVecInstance<'parent, 'a, T> {
    pub fn as_mut(&mut self) -> &mut Vec<T::Item<'a>> {
        self.inner.as_mut().expect("None only after drop")
    }
}

impl<'parent, 'a, T: TempVecType> Drop for TemporaryVecInstance<'parent, 'a, T> {
    fn drop(&mut self) {
        let mut vec = self.inner.take().expect("None only after drop");

        vec.clear();

        // Transmute temporary lifetime back to 'static
        // Should be safe since vector is empty
        let vec = unsafe { mem::transmute(vec) };

        *self.parent = Some(vec);
    }
}
