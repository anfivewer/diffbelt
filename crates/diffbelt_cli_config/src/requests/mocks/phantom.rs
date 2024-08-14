use diffbelt_protos::protos::api::phantom::{
    StartPhantomRequest, StartPhantomRequestArgs, StartPhantomResponse,
};
use diffbelt_protos::{Serialized, Serializer};
use lexpr::from_str;
use serde::Deserialize;
use std::str::from_utf8;
use thiserror::Error;

fn default_as_false() -> bool {
    false
}

#[derive(Deserialize)]
struct StartPhantomRequestMock {
    generation_id: String,
    #[serde(default = "default_as_false")]
    generation_id_base64: bool,
}

#[derive(Deserialize)]
struct StartPhantomResponseMock {
    phantom_id: String,
    phantom_id_base64: bool,
}

#[derive(Error, Debug)]
pub enum PhantomSerializationError {
    #[error("Not base64")]
    NotBase64,
    #[error("EmptyPhantomId")]
    EmptyPhantomId,
}

impl StartPhantomRequestMock {
    pub fn to_flatbuffers(
        &self,
    ) -> Result<Serialized<StartPhantomRequest>, PhantomSerializationError> {
        let mut serializer = Serializer::new();

        let generation_id = if self.generation_id_base64 {
            let bytes = base64::decode(self.generation_id.as_str())
                .map_err(|_| PhantomSerializationError::NotBase64)?;
            serializer.create_vector(&bytes)
        } else {
            serializer.create_vector(self.generation_id.as_bytes())
        };

        let wip = StartPhantomRequest::create(
            serializer.buffer_builder(),
            &StartPhantomRequestArgs {
                generation_id: Some(generation_id),
            },
        );

        Ok(serializer.finish(wip))
    }
}

impl StartPhantomResponseMock {
    pub fn from_flatbuffers(
        data: &StartPhantomResponse,
    ) -> Result<Self, PhantomSerializationError> {
        let phantom_id = data
            .phantom_id()
            .ok_or(PhantomSerializationError::EmptyPhantomId)?
            .bytes();

        match from_utf8(phantom_id) {
            Ok(phantom_id) => Ok(Self {
                phantom_id: phantom_id.to_string(),
                phantom_id_base64: false,
            }),
            Err(_) => Ok(Self {
                phantom_id: base64::encode(phantom_id),
                phantom_id_base64: true,
            }),
        }
    }
}
