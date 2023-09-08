use crate::YamlNode;
use std::cell::RefCell;
use std::ops::DerefMut;
use std::rc::Rc;

thread_local! {
    static YAML_NODE_TRESPASS: RefCell<(u64, Option<Rc<YamlNode>>)> = RefCell::new((0, None));
}

pub fn save_yaml_node(node: Rc<YamlNode>) -> u64 {
    YAML_NODE_TRESPASS.with(|value| {
        let mut value = value.borrow_mut();
        let value = value.deref_mut();

        let counter = value.0 + 1;
        value.0 = counter;

        value.1 = Some(node);

        counter
    })
}

pub fn take_yaml_node(expected_counter: u64) -> Option<Rc<YamlNode>> {
    YAML_NODE_TRESPASS.with(|value| {
        let mut value = value.borrow_mut();
        let (actual_counter, value) = value.deref_mut();

        if *actual_counter != expected_counter {
            return None;
        }

        *actual_counter += 1;

        value.take()
    })
}
