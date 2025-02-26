use crate::protos::api::collection::{CreateCollectionRequest, CreateCollectionResponse};
use crate::protos::api::methods::{Request, Response};
use crate::protos::api::phantom::StartPhantomRequest;
use crate::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateMapMultiInput, AggregateMapMultiOutput, AggregateReduceInput,
    AggregateTargetInfo,
};
use crate::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiOutput, RecordUpdate,
};
use diffbelt_protos::protos::api::common::ErrorResponse;

#[macro_export]
macro_rules! flatbuffers_generic {
    ($name:ident, $proto_name:ident) => {
        pub struct $proto_name;
        impl ::diffbelt_protos::FlatbuffersGenericType for $proto_name {
            type FlatType<'a> = $name<'a>;
            fn name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

flatbuffers_generic!(Request, RequestProto);
flatbuffers_generic!(Response, ResponseProto);
flatbuffers_generic!(ErrorResponse, ErrorResponseProto);
flatbuffers_generic!(CreateCollectionRequest, CreateCollectionRequestProto);
flatbuffers_generic!(CreateCollectionResponse, CreateCollectionResponseProto);
flatbuffers_generic!(RecordUpdate, RecordUpdateProto);
flatbuffers_generic!(MapFilterMultiInput, MapFilterMultiInputProto);
flatbuffers_generic!(AggregateMapMultiInput, AggregateMapMultiInputProto);
flatbuffers_generic!(AggregateTargetInfo, AggregateTargetInfoProto);
flatbuffers_generic!(AggregateReduceInput, AggregateReduceInputProto);
flatbuffers_generic!(MapFilterMultiOutput, MapFilterMultiOutputProto);
flatbuffers_generic!(AggregateMapMultiOutput, AggregateMapMultiOutputProto);
flatbuffers_generic!(AggregateApplyOutput, AggregateApplyOutputProto);
flatbuffers_generic!(StartPhantomRequest, StartPhantomRequestProto);
