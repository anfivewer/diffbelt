use crate::base::error::TransformError;
use enum_as_inner::EnumAsInner;
use enum_kinds::EnumKind;

#[derive(EnumAsInner, EnumKind)]
#[enum_kind(HandlerContextKind)]
pub enum HandlerContext {
    None,
    Map(MapContext),
}

pub struct MapContext {
    pub bytes_to_free: usize,
}

pub trait HandlerContextMapError<T> {
    fn map_err_self_to_transform_err(self, expected: HandlerContextKind) -> Result<T, TransformError>;
}

impl<T> HandlerContextMapError<T> for Result<T, HandlerContext> {
    fn map_err_self_to_transform_err(self, expected: HandlerContextKind) -> Result<T, TransformError> {
        match self {
            Ok(x) => Ok(x),
            Err(err) => Err(TransformError::Unspecified(format!(
                "Invalid map state, expected {expected:?}, got {:?}",
                HandlerContextKind::from(err)
            ))),
        }
    }
}
