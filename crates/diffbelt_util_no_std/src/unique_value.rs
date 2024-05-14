pub struct UniqueValue;

impl PartialEq for UniqueValue {
    fn eq(&self, _other: &Self) -> bool {
        false
    }

    fn ne(&self, _other: &Self) -> bool {
        true
    }
}
