use core::marker::PhantomData;

pub struct OnDrop<P, F: FnOnce(P)> {
    fun: Option<(F, P)>,
    phantom: PhantomData<P>,
}

impl<P, F: FnOnce(P)> OnDrop<P, F> {
    pub fn new(f: F, params: P) -> Self {
        Self {
            fun: Some((f, params)),
            phantom: Default::default(),
        }
    }

    pub fn params_mut(&mut self) -> &mut P {
        let (_, params) = self.fun.as_mut().expect("should exists before drop");
        params
    }
}

impl<P, F: FnOnce(P)> Drop for OnDrop<P, F> {
    fn drop(&mut self) {
        if let Some((f, p)) = self.fun.take() {
            f(p);
        }
    }
}
