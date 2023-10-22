use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub trait Wrap<A>: Sized {
    fn wrap(data: A) -> Self;
}

impl<T> Wrap<T> for T {
    fn wrap(data: T) -> Self {
        data
    }
}

impl<T> Wrap<T> for Rc<RefCell<T>> {
    fn wrap(data: T) -> Self {
        Rc::new(RefCell::new(data))
    }
}

impl<T> Wrap<T> for Arc<Mutex<T>> {
    fn wrap(data: T) -> Self {
        Arc::new(Mutex::new(data))
    }
}
