use crate::serde::error::YamlDecodingError;
use crate::{YamlNode, YamlSequence};
use serde::de::{DeserializeSeed, SeqAccess};
use std::rc::Rc;

pub struct YamlSequenceDe<'de, I: Iterator<Item = &'de Rc<YamlNode>>> {
    pub sequence: &'de YamlSequence,
    pub iter: I,
}

impl<'de, I: Iterator<Item = &'de Rc<YamlNode>>> SeqAccess<'de> for YamlSequenceDe<'de, I> {
    type Error = YamlDecodingError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let Some(value) = self.iter.next() else {
            return Ok(None);
        };

        let de = super::Deserializer::from_yaml_node(value);

        seed.deserialize(de).map(Some)
    }
}
