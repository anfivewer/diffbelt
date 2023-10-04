use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::InputType;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;

macro_rules! input_type_into_diffbelt {
    ( $method_name:ident, $t:ty, $body_variant:ident ) => {
        pub fn $method_name(self) -> Result<DiffbeltCallInput<$t>, TransformError> {
            let InputType::DiffbeltCall(call) = self else {
                return Err(TransformError::Unspecified(
                    "Unexpected input, expected DiffbeltCall".to_string(),
                ));
            };

            let DiffbeltCallInput { body } = call;

            let DiffbeltResponseBody::$body_variant(body) = body else {
                return Err(TransformError::Unspecified(format!(
                    "Unexpected input, expected {}",
                    stringify!($body_variant)
                )));
            };

            Ok(DiffbeltCallInput { body })
        }
    };
}

impl InputType {
    input_type_into_diffbelt!(into_diffbelt_ok, (), Ok);
    input_type_into_diffbelt!(into_diffbelt_diff, DiffCollectionResponseJsonData, Diff);
}
