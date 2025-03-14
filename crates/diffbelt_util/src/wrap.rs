use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

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

impl<T> Wrap<T> for Arc<T> {
    fn wrap(data: T) -> Self {
        Arc::new(data)
    }
}

impl<T> Wrap<T> for Arc<std::sync::Mutex<T>> {
    fn wrap(data: T) -> Self {
        Arc::new(Wrap::wrap(data))
    }
}

impl<T> Wrap<T> for Arc<tokio::sync::Mutex<T>> {
    fn wrap(data: T) -> Self {
        Arc::new(Wrap::wrap(data))
    }
}

impl<T> Wrap<T> for std::sync::Mutex<T> {
    fn wrap(data: T) -> Self {
        std::sync::Mutex::new(data)
    }
}

impl<T> Wrap<T> for tokio::sync::Mutex<T> {
    fn wrap(data: T) -> Self {
        tokio::sync::Mutex::new(data)
    }
}
