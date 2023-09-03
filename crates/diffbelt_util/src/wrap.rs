use std::cell::RefCell;
use std::rc::Rc;

pub trait Wrap<A>: Sized {
    fn wrap(data: A) -> Self;
}

impl<T> Wrap<T> for Rc<RefCell<T>> {
    fn wrap(data: T) -> Self {
        Rc::new(RefCell::new(data))
    }
}
