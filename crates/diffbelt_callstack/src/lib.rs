#![no_std]

extern crate alloc;
extern crate self as diffbelt_callstack;

use core::fmt::{Display, Formatter};

#[macro_export]
macro_rules! callstack {
    ($root:literal) => {{
        ::diffbelt_callstack::CallStack {
            name: $root,
            parent: None,
        }
    }};
    ($parent:expr, $name:literal) => {{
        ::diffbelt_callstack::CallStack::new($name, &$parent)
    }};
}

#[derive(Copy, Clone, Debug)]
pub struct CallStack<'a> {
    name: &'static str,
    parent: Option<&'a CallStack<'a>>,
}

impl<'a> CallStack<'a> {
    pub fn new(name: &'static str, parent: &'a CallStack<'a>) -> Self {
        Self {
            name,
            parent: Some(parent),
        }
    }
}

impl Display for CallStack<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        () = f.write_str(self.name)?;

        let mut node = self.parent;
        while let Some(parent) = node {
            () = f.write_str(" <- ")?;
            () = f.write_str(parent.name)?;

            node = parent.parent;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::CallStack;
    use alloc::format;

    #[test]
    fn callstack_new() {
        let cs = callstack!("root");

        callstack_a(callstack!(cs, "a"));
    }

    fn callstack_a(cs: CallStack) {
        assert_eq!(format!("{cs}"), "a <- root");
        callstack_b(callstack!(cs, "b"));
        callstack_c(callstack!(cs, "c"));
    }

    fn callstack_b(cs: CallStack) {
        assert_eq!(format!("{cs}"), "b <- a <- root");
    }

    fn callstack_c(cs: CallStack) {
        assert_eq!(format!("{cs}"), "c <- a <- root");
    }
}
