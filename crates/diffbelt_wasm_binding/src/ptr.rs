#[derive(Copy, Clone, Debug)]
pub struct NativePtrImpl;

pub trait PtrImpl {
    type Ptr<T: Clone>: Copy;
    type MutPtr<T: Clone>: Copy;
}

impl PtrImpl for NativePtrImpl {
    type Ptr<T: Clone> = *const T;
    type MutPtr<T: Clone> = *mut T;
}