use crate::protos::api::collection::{
    CreateCollectionRequest, CreateCollectionRequestArgs, CreateCollectionResponse,
    CreateCollectionResponseArgs,
};
use crate::protos::api::common::ErrorResponseArgs;
use crate::protos::api::generation::{
    CommitGenerationRequest, CommitGenerationRequestArgs, CommitGenerationResponse,
    CommitGenerationResponseArgs, GenerationIdStreamRequest, GenerationIdStreamRequestArgs,
    GenerationIdStreamResponse, GenerationIdStreamResponseArgs, StartGenerationRequest,
    StartGenerationRequestArgs, StartGenerationResponse, StartGenerationResponseArgs,
};
use crate::protos::api::get::{GetRequest, GetRequestArgs, GetResponse, GetResponseArgs};
use crate::protos::api::get_keys_around::{
    GetKeysAroundRequest, GetKeysAroundRequestArgs, GetKeysAroundResponse,
    GetKeysAroundResponseArgs,
};
use crate::protos::api::methods::{Request, RequestArgs, Response, ResponseArgs};
use crate::protos::api::phantom::{StartPhantomRequest, StartPhantomRequestArgs};
use crate::protos::api::put_many::{
    PutManyRequest, PutManyRequestArgs, PutManyResponse, PutManyResponseArgs,
};
use crate::protos::api::query::{
    NextQueryRequest, NextQueryRequestArgs, QueryResponse, QueryResponseArgs, StartQueryRequest,
    StartQueryRequestArgs,
};
use crate::protos::api::readers::{
    CreateReaderRequest, CreateReaderRequestArgs, CreateReaderResponse, CreateReaderResponseArgs,
    ListReadersRequest, ListReadersRequestArgs, ListReadersResponse, ListReadersResponseArgs,
};
use crate::protos::transform::aggregate::{
    AggregateApplyOutput, AggregateApplyOutputArgs, AggregateMapMultiInput,
    AggregateMapMultiInputArgs, AggregateMapMultiOutput, AggregateMapMultiOutputArgs,
    AggregateReduceInput, AggregateTargetInfo, AggregateTargetInfoArgs,
};
use crate::protos::transform::map_filter::{
    MapFilterMultiInput, MapFilterMultiInputArgs, MapFilterMultiOutput, MapFilterMultiOutputArgs,
    RecordUpdate, RecordUpdateArgs,
};
use diffbelt_protos::protos::api::common::ErrorResponse;
pub use paste::paste;

#[macro_export]
macro_rules! flatbuffers_generic {
    ($name:ident, $args:ty) => {
        ::diffbelt_protos::protos::impls::paste! {
            pub struct [<$name Proto>];
        }
        impl ::diffbelt_protos::FlatbuffersGenericType for ::diffbelt_protos::protos::impls::paste! { [<$name Proto>] } {
            type FlatType<'a> = $name<'a>;
            type FlatArgs<'a> = $args;
            fn name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

flatbuffers_generic!(Request, RequestArgs);
flatbuffers_generic!(Response, ResponseArgs);
flatbuffers_generic!(ErrorResponse, ErrorResponseArgs<'a>);
flatbuffers_generic!(CreateCollectionRequest, CreateCollectionRequestArgs<'a>);
flatbuffers_generic!(CreateCollectionResponse, CreateCollectionResponseArgs<'a>);
flatbuffers_generic!(PutManyRequest, PutManyRequestArgs<'a>);
flatbuffers_generic!(PutManyResponse, PutManyResponseArgs<'a>);
flatbuffers_generic!(StartGenerationRequest, StartGenerationRequestArgs<'a>);
flatbuffers_generic!(StartGenerationResponse, StartGenerationResponseArgs);
flatbuffers_generic!(CommitGenerationRequest, CommitGenerationRequestArgs<'a>);
flatbuffers_generic!(CommitGenerationResponse, CommitGenerationResponseArgs);
flatbuffers_generic!(RecordUpdate, RecordUpdateArgs<'a>);
flatbuffers_generic!(MapFilterMultiInput, MapFilterMultiInputArgs<'a>);
flatbuffers_generic!(AggregateMapMultiInput, AggregateMapMultiInputArgs<'a>);
flatbuffers_generic!(AggregateTargetInfo, AggregateTargetInfoArgs<'a>);
flatbuffers_generic!(AggregateReduceInput, AggregateReduceInput<'a>);
flatbuffers_generic!(MapFilterMultiOutput, MapFilterMultiOutputArgs<'a>);
flatbuffers_generic!(AggregateMapMultiOutput, AggregateMapMultiOutputArgs<'a>);
flatbuffers_generic!(AggregateApplyOutput, AggregateApplyOutputArgs<'a>);
flatbuffers_generic!(StartPhantomRequest, StartPhantomRequestArgs<'a>);
flatbuffers_generic!(GenerationIdStreamRequest, GenerationIdStreamRequestArgs<'a>);
flatbuffers_generic!(
    GenerationIdStreamResponse,
    GenerationIdStreamResponseArgs<'a>
);
flatbuffers_generic!(ListReadersRequest, ListReadersRequestArgs<'a>);
flatbuffers_generic!(ListReadersResponse, ListReadersResponseArgs<'a>);
flatbuffers_generic!(CreateReaderRequest, CreateReaderRequestArgs<'a>);
flatbuffers_generic!(CreateReaderResponse, CreateReaderResponseArgs);
flatbuffers_generic!(StartQueryRequest, StartQueryRequestArgs<'a>);
flatbuffers_generic!(NextQueryRequest, NextQueryRequestArgs<'a>);
flatbuffers_generic!(QueryResponse, QueryResponseArgs<'a>);
flatbuffers_generic!(GetRequest, GetRequestArgs<'a>);
flatbuffers_generic!(GetResponse, GetResponseArgs<'a>);
flatbuffers_generic!(GetKeysAroundRequest, GetKeysAroundRequestArgs<'a>);
flatbuffers_generic!(GetKeysAroundResponse, GetKeysAroundResponseArgs<'a>);
