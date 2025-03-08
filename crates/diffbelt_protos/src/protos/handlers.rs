use crate::protos::api::collection::{CreateCollectionRequestArgs, CreateCollectionResponseArgs};
use crate::protos::api::generation::{
    CommitGenerationRequestArgs, CommitGenerationResponseArgs, StartGenerationRequestArgs,
    StartGenerationResponseArgs,
};
use crate::protos::api::methods::{
    Request, RequestArgs, RequestBody, Response, ResponseArgs, ResponseBody,
};
use crate::protos::api::put_many::{PutManyRequestArgs, PutManyResponseArgs};
use crate::protos::impls::{
    CommitGenerationRequestProto, CommitGenerationResponseProto, CreateCollectionRequestProto,
    CreateCollectionResponseProto, PutManyRequestProto, PutManyResponseProto, RequestProto,
    ResponseProto, StartGenerationRequestProto, StartGenerationResponseProto,
};
use crate::{FlatbuffersGenericType, OwnedSerialized, Serializer};

pub trait ApiHandler {
    type FlatbuffersRequest: FlatbuffersGenericType;
    type FlatbuffersRequestArgs<'a>;
    type FlatbuffersResponse: FlatbuffersGenericType;
    type FlatbuffersResponseArgs<'a>;

    fn request<'a>(
        request: &'a <RequestProto as FlatbuffersGenericType>::FlatType<'a>,
    ) -> Option<<Self::FlatbuffersRequest as FlatbuffersGenericType>::FlatType<'a>>;
    fn create_request<'a>(
        serializer: Serializer<'a, RequestProto>,
        request: Self::FlatbuffersRequestArgs<'a>,
    ) -> OwnedSerialized<RequestProto>;
    fn response(
        response: <ResponseProto as FlatbuffersGenericType>::FlatType<'_>,
    ) -> Option<<Self::FlatbuffersResponse as FlatbuffersGenericType>::FlatType<'_>>;
    fn create_response<'a>(
        serializer: Serializer<'a, ResponseProto>,
        response: Self::FlatbuffersResponseArgs<'a>,
    ) -> OwnedSerialized<ResponseProto>;
}

macro_rules! api_handler {
    (
        $struct_name:tt,
        $body_type:ident,
        $union_method:ident,
        request = $request:ident,
        $request_args:ident,
        response = $response:ident,
        $response_args:ty,
    ) => {
        pub struct $struct_name;

        impl ApiHandler for $struct_name {
            type FlatbuffersRequest = $request;
            type FlatbuffersRequestArgs<'a> = $request_args<'a>;
            type FlatbuffersResponse = $response;
            type FlatbuffersResponseArgs<'a> = $response_args;

            fn request<'a>(
                request: &'a <RequestProto as FlatbuffersGenericType>::FlatType<'a>,
            ) -> Option<<Self::FlatbuffersRequest as FlatbuffersGenericType>::FlatType<'a>> {
                request.$union_method()
            }

            fn create_request<'a>(
                mut serializer: Serializer<'a, RequestProto>,
                request: Self::FlatbuffersRequestArgs<'a>,
            ) -> OwnedSerialized<RequestProto> {
                let request =
                    <Self::FlatbuffersRequest as FlatbuffersGenericType>::FlatType::create(
                        serializer.buffer_builder(),
                        &request,
                    );
                let request = Request::create(
                    serializer.buffer_builder(),
                    &RequestArgs {
                        body_type: RequestBody::$body_type,
                        body: Some(request.as_union_value()),
                    },
                );
                serializer.finish(request)
            }

            fn response(
                response: <ResponseProto as FlatbuffersGenericType>::FlatType<'_>,
            ) -> Option<<Self::FlatbuffersResponse as FlatbuffersGenericType>::FlatType<'_>> {
                response.$union_method()
            }

            fn create_response<'a>(
                mut serializer: Serializer<'a, ResponseProto>,
                response: Self::FlatbuffersResponseArgs<'a>,
            ) -> OwnedSerialized<ResponseProto> {
                let response =
                    <Self::FlatbuffersResponse as FlatbuffersGenericType>::FlatType::create(
                        serializer.buffer_builder(),
                        &response,
                    );
                let response = Response::create(
                    serializer.buffer_builder(),
                    &ResponseArgs {
                        body_type: ResponseBody::$body_type,
                        body: Some(response.as_union_value()),
                    },
                );
                serializer.finish(response)
            }
        }
    };
}

api_handler!(
    CreateCollectionApiHandler,
    CreateCollection,
    body_as_create_collection,
    request = CreateCollectionRequestProto,
    CreateCollectionRequestArgs,
    response = CreateCollectionResponseProto,
    CreateCollectionResponseArgs<'a>,
);
api_handler!(
    StartGenerationApiHandler,
    StartGeneration,
    body_as_start_generation,
    request = StartGenerationRequestProto,
    StartGenerationRequestArgs,
    response = StartGenerationResponseProto,
    StartGenerationResponseArgs,
);
api_handler!(
    CommitGenerationApiHandler,
    CommitGeneration,
    body_as_commit_generation,
    request = CommitGenerationRequestProto,
    CommitGenerationRequestArgs,
    response = CommitGenerationResponseProto,
    CommitGenerationResponseArgs,
);
api_handler!(
    PutManyApiHandler,
    PutMany,
    body_as_put_many,
    request = PutManyRequestProto,
    PutManyRequestArgs,
    response = PutManyResponseProto,
    PutManyResponseArgs<'a>,
);
