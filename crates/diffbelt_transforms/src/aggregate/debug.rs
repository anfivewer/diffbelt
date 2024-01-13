use crate::aggregate::state::State;
use crate::aggregate::AggregateTransform;

impl AggregateTransform {
    pub fn debug_print(&self) {
        println!("State: {:#?}", self.state);

        if let State::Processing(state) = &self.state {
            println!("Target keys:");

            for (key, value) in &state.target_keys {
                println!("Key: {key:?}\nValue: {value:#?}\n");
            }
        }
    }
}
