#[derive(Copy, Clone, Debug)]
pub struct NativePtrImpl;

pub trait PtrImpl {
    type Ptr<T: Copy>: Copy;
}

impl PtrImpl for NativePtrImpl {
    type Ptr<T: Copy> = *mut T;
}