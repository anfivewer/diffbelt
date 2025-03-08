use crate::protos::api::collection::{CreateCollectionRequest, CreateCollectionResponse};
use crate::protos::api::generation::{
    CommitGenerationRequest, CommitGenerationResponse, StartGenerationRequest,
    StartGenerationResponse,
};
use crate::protos::api::methods::{Request, Response};
use crate::protos::api::phantom::StartPhantomRequest;
use crate::protos::api::put_many::{PutManyRequest, PutManyResponse};
use crate::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};
use crate::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, RecordUpdate,
};
use diffbelt_protos::protos::api::common::ErrorResponse;
pub use paste::paste;

#[macro_export]
macro_rules! flatbuffers_generic {
    ($name:ident) => {
        ::diffbelt_protos::protos::impls::paste! {
            pub struct [<$name Proto>];
        }
        impl ::diffbelt_protos::FlatbuffersGenericType for ::diffbelt_protos::protos::impls::paste! { [<$name Proto>] } {
            type FlatType<'a> = $name<'a>;
            fn name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

flatbuffers_generic!(Request);
flatbuffers_generic!(Response);
flatbuffers_generic!(ErrorResponse);
flatbuffers_generic!(CreateCollectionRequest);
flatbuffers_generic!(CreateCollectionResponse);
flatbuffers_generic!(PutManyRequest);
flatbuffers_generic!(PutManyResponse);
flatbuffers_generic!(StartGenerationRequest);
flatbuffers_generic!(StartGenerationResponse);
flatbuffers_generic!(CommitGenerationRequest);
flatbuffers_generic!(CommitGenerationResponse);
flatbuffers_generic!(RecordUpdate);
flatbuffers_generic!(MapFilterMultiInput);
flatbuffers_generic!(AggregateMapMultiInput);
flatbuffers_generic!(AggregateTargetInfo);
flatbuffers_generic!(AggregateReduceInput);
flatbuffers_generic!(MapFilterMultiOutput);
flatbuffers_generic!(AggregateMapMultiOutput);
flatbuffers_generic!(AggregateApplyOutput);
flatbuffers_generic!(StartPhantomRequest);
