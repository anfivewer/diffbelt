use hashbrown::Equivalent;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

#[derive(Eq, PartialEq, Clone)]
pub struct ArcStringPair(pub Arc<str>, pub Arc<str>);

pub struct ArcStringPairRef<'a>(pub &'a str, pub &'a str);

impl Hash for ArcStringPair {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.0.as_bytes());
        state.write(self.1.as_bytes());
    }
}

impl Hash for ArcStringPairRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write(self.0.as_bytes());
        state.write(self.1.as_bytes());
    }
}

impl Equivalent<ArcStringPair> for ArcStringPairRef<'_> {
    fn equivalent(&self, key: &ArcStringPair) -> bool {
        self.0 == key.0.as_ref() && self.1 == key.1.as_ref()
    }
}
