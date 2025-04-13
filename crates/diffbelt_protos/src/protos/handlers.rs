use crate::protos::api::methods::{
    Request, RequestArgs, RequestBody, Response, ResponseArgs, ResponseBody,
};
use crate::protos::impls::{
    CommitGenerationRequestProto, CommitGenerationResponseProto, CreateCollectionRequestProto,
    CreateCollectionResponseProto, CreateReaderRequestProto, CreateReaderResponseProto,
    GenerationIdStreamRequestProto, GenerationIdStreamResponseProto, GetKeysAroundRequestProto,
    GetKeysAroundResponseProto, GetRequestProto, GetResponseProto, ListReadersRequestProto,
    ListReadersResponseProto, NextQueryRequestProto, PutManyRequestProto, PutManyResponseProto,
    QueryResponseProto, RequestProto, ResponseProto, StartGenerationRequestProto,
    StartGenerationResponseProto, StartQueryRequestProto,
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

pub type ApiHandlerRequestFlatType<'a, T: ApiHandler> =
    <T::FlatbuffersRequest as FlatbuffersGenericType>::FlatType<'a>;

macro_rules! api_handler {
    (
        $struct_name:tt,
        $body_type:ident,
        $union_method:ident,
        request = $request:ident,
        response = $response:ident,
    ) => {
        pub struct $struct_name;

        impl ApiHandler for $struct_name {
            type FlatbuffersRequest = $request;
            type FlatbuffersRequestArgs<'a> = <$request as FlatbuffersGenericType>::FlatArgs<'a>;
            type FlatbuffersResponse = $response;
            type FlatbuffersResponseArgs<'a> = <$response as FlatbuffersGenericType>::FlatArgs<'a>;

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
    response = CreateCollectionResponseProto,
);
api_handler!(
    StartGenerationApiHandler,
    StartGeneration,
    body_as_start_generation,
    request = StartGenerationRequestProto,
    response = StartGenerationResponseProto,
);
api_handler!(
    CommitGenerationApiHandler,
    CommitGeneration,
    body_as_commit_generation,
    request = CommitGenerationRequestProto,
    response = CommitGenerationResponseProto,
);
api_handler!(
    PutManyApiHandler,
    PutMany,
    body_as_put_many,
    request = PutManyRequestProto,
    response = PutManyResponseProto,
);
api_handler!(
    GenerationIdStreamApiHandler,
    GenerationIdStream,
    body_as_generation_id_stream,
    request = GenerationIdStreamRequestProto,
    response = GenerationIdStreamResponseProto,
);
api_handler!(
    ListReadersApiHandler,
    ListReaders,
    body_as_list_readers,
    request = ListReadersRequestProto,
    response = ListReadersResponseProto,
);
api_handler!(
    CreateReaderApiHandler,
    CreateReader,
    body_as_create_reader,
    request = CreateReaderRequestProto,
    response = CreateReaderResponseProto,
);
api_handler!(
    StartQueryApiHandler,
    StartQuery,
    body_as_start_query,
    request = StartQueryRequestProto,
    response = QueryResponseProto,
);
api_handler!(
    NextQueryApiHandler,
    NextQuery,
    body_as_next_query,
    request = NextQueryRequestProto,
    response = QueryResponseProto,
);
api_handler!(
    GetApiHandler,
    Get,
    body_as_get,
    request = GetRequestProto,
    response = GetResponseProto,
);
api_handler!(
    GetKeysAroundApiHandler,
    GetKeysAround,
    body_as_get_keys_around,
    request = GetKeysAroundRequestProto,
    response = GetKeysAroundResponseProto,
);
