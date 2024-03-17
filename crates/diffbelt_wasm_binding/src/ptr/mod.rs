use bytemuck::{Pod, Zeroable};
use core::marker::PhantomData;

pub mod bytes;
pub mod slice;

#[derive(Pod, Zeroable, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct NativePtrImpl;

pub trait PtrImpl {
    type Ptr<T: Pod>: Pod;
    type MutPtr<T: Pod>: Pod;
}

impl PtrImpl for NativePtrImpl {
    type Ptr<T: Pod> = ConstPtr<T>;
    type MutPtr<T: Pod> = MutPtr<T>;
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(transparent)]
pub struct ConstPtr<T: Pod> {
    value: i32,
    phantom: PhantomData<T>,
}

#[derive(Pod, Zeroable, Copy, Clone)]
#[repr(transparent)]
pub struct MutPtr<T: Pod> {
    value: i32,
    phantom: PhantomData<T>,
}

impl<T: Pod> From<*const T> for ConstPtr<T> {
    fn from(value: *const T) -> Self {
        Self {
            value: value as i32,
            phantom: Default::default(),
        }
    }
}

impl<T: Pod> From<MutPtr<T>> for ConstPtr<T> {
    fn from(value: MutPtr<T>) -> Self {
        Self {
            value: value.value,
            phantom: Default::default(),
        }
    }
}

impl<T: Pod> From<*mut T> for MutPtr<T> {
    fn from(value: *mut T) -> Self {
        Self {
            value: value as i32,
            phantom: Default::default(),
        }
    }
}

impl<T: Pod> ConstPtr<T> {
    pub fn as_ptr(self) -> *const T {
        self.value as *const T
    }
}

impl<T: Pod> MutPtr<T> {
    pub fn as_mut_ptr(self) -> *mut T {
        self.value as *mut T
    }
}
