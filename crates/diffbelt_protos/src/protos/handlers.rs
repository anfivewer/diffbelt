use crate::protos::api::collection::{
    CreateCollectionRequest, CreateCollectionRequestArgs, CreateCollectionResponse,
    CreateCollectionResponseArgs,
};
use crate::protos::api::methods::{
    Request, RequestArgs, RequestBody, Response, ResponseArgs, ResponseBody,
};
use crate::protos::impls::{
    CreateCollectionRequestProto, CreateCollectionResponseProto, RequestProto, ResponseProto,
};
use crate::{FlatbuffersGenericType, OwnedSerialized, Serializer};
use alloc::string::String;

pub trait ApiHandler {
    type Params;
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

pub struct CreateCollectionApiHandler;

impl ApiHandler for CreateCollectionApiHandler {
    type Params = ();
    type FlatbuffersRequest = CreateCollectionRequestProto;
    type FlatbuffersRequestArgs<'a> = CreateCollectionRequestArgs<'a>;
    type FlatbuffersResponse = CreateCollectionResponseProto;
    type FlatbuffersResponseArgs<'a> = CreateCollectionResponseArgs<'a>;

    fn request<'a>(
        request: &'a <RequestProto as FlatbuffersGenericType>::FlatType<'a>,
    ) -> Option<<Self::FlatbuffersRequest as FlatbuffersGenericType>::FlatType<'a>> {
        request.body_as_create_collection()
    }

    fn create_request<'a>(
        mut serializer: Serializer<'a, RequestProto>,
        request: Self::FlatbuffersRequestArgs<'a>,
    ) -> OwnedSerialized<RequestProto> {
        let request = <Self::FlatbuffersRequest as FlatbuffersGenericType>::FlatType::create(
            serializer.buffer_builder(),
            &request,
        );
        let request = Request::create(
            serializer.buffer_builder(),
            &RequestArgs {
                body_type: RequestBody::CreateCollection,
                body: Some(request.as_union_value()),
            },
        );
        serializer.finish(request).into_owned()
    }

    fn response(
        response: <ResponseProto as FlatbuffersGenericType>::FlatType<'_>,
    ) -> Option<<Self::FlatbuffersResponse as FlatbuffersGenericType>::FlatType<'_>> {
        response.body_as_create_collection()
    }

    fn create_response<'a>(
        mut serializer: Serializer<'a, ResponseProto>,
        response: Self::FlatbuffersResponseArgs<'a>,
    ) -> OwnedSerialized<ResponseProto> {
        let response = <Self::FlatbuffersResponse as FlatbuffersGenericType>::FlatType::create(
            serializer.buffer_builder(),
            &response,
        );
        let response = Response::create(
            serializer.buffer_builder(),
            &ResponseArgs {
                body_type: ResponseBody::CreateCollection,
                body: Some(response.as_union_value()),
            },
        );
        serializer.finish(response).into_owned()
    }
}
