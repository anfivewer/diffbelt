use crate::transforms::Transform;
use crate::Collection;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::protos::api::collection::CreateCollectionRequestArgs;
use diffbelt_protos::protos::api::common::ErrorResponse;
use diffbelt_protos::protos::api::readers::{
    CreateReaderRequestArgs, CreateReaderResponse, ListReadersRequestArgs, ReaderRecord,
    ReaderRecordArgs,
};
use diffbelt_protos::protos::handlers::{
    ApiHandler, CreateCollectionApiHandler, CreateReaderApiHandler, ListReadersApiHandler,
};
use diffbelt_protos::protos::impls::RequestProto;
use diffbelt_protos::Serializer;
use std::collections::HashMap;
use thiserror::Error;

pub struct InitCollectionsOptions<
    'a,
    PrintBeforeCreateFn: Fn(&Collection) -> (),
    PrintAfterCreateFn: Fn(&Collection) -> (),
> {
    pub client: &'a DiffbeltClient,
    pub collections: &'a [Collection],
    pub transforms: &'a [Transform],
    pub print_before_create: PrintBeforeCreateFn,
    pub print_after_create: PrintAfterCreateFn,
}

#[derive(Error, Debug)]
pub enum InitCollectionsError {
    #[error("Message({0})")]
    Message(String),
    #[error(transparent)]
    DiffbeltClient(#[from] DiffbeltClientError),
}

pub async fn init_collections<
    PrintBeforeCreateFn: Fn(&Collection) -> (),
    PrintAfterCreateFn: Fn(&Collection) -> (),
>(
    options: InitCollectionsOptions<'_, PrintBeforeCreateFn, PrintAfterCreateFn>,
) -> Result<(), InitCollectionsError> {
    let client = options.client;

    let response = client.list_collections().await?;

    let mut existing_collections_is_manual = HashMap::with_capacity(response.items.len());
    for item in &response.items {
        existing_collections_is_manual.insert(item.name.as_str(), item.is_manual);
    }

    for collection in options.collections {
        let existing = existing_collections_is_manual.get(collection.name.as_ref());
        if let Some(is_manual) = existing {
            if collection.manual == *is_manual {
                let () = init_collection_readers(collection, options.transforms, client).await?;

                continue;
            } else if *is_manual {
                return Err(InitCollectionsError::Message(format!(
                    "Collection {} already exists and is manual, but should not",
                    collection.name
                )));
            } else {
                return Err(InitCollectionsError::Message(format!(
                    "Collection {} already exists and is not manual, but should be",
                    collection.name
                )));
            }
        }

        (&options.print_before_create)(collection);

        let mut serializer = Serializer::<RequestProto>::new();
        let collection_name = Some(serializer.create_string(collection.name.as_ref()));
        let request = CreateCollectionApiHandler::create_request(
            serializer,
            CreateCollectionRequestArgs {
                collection_name,
                is_manual: collection.manual,
            },
        );
        let response = client
            .flatbuffers_call::<CreateCollectionApiHandler>(request)
            .await?;

        match response.response() {
            Ok(_) => (),
            Err(Some(err)) => {
                return Err(InitCollectionsError::Message(format!(
                    "Error code {}, reason: {}, details: {}",
                    err.code(),
                    err.reason().unwrap_or("()"),
                    err.details().unwrap_or("()"),
                )));
            }
            Err(None) => {
                return Err(InitCollectionsError::Message(String::from("No response")));
            }
        };

        (&options.print_after_create)(collection);

        let () = init_collection_readers(collection, options.transforms, client).await?;
    }

    Ok(())
}

async fn init_collection_readers(
    collection: &Collection,
    transforms: &[Transform],
    client: &DiffbeltClient,
) -> Result<(), InitCollectionsError> {
    let mut serializer = Serializer::new();
    let collection_name = Some(serializer.create_string(&collection.name));
    let request = ListReadersApiHandler::create_request(
        serializer,
        ListReadersRequestArgs { collection_name },
    );
    let response = client
        .flatbuffers_call::<ListReadersApiHandler>(request)
        .await?;
    let response = response
        .response()
        .map_err(map_error_response_to_init_collection_error)?;

    let readers = response.readers().unwrap_or_default();
    let mut existing_readers = HashMap::with_capacity(readers.len());

    for reader in readers {
        let Some(name) = reader.reader_name() else {
            continue;
        };
        existing_readers.insert(name, reader);
    }

    let mut buffer = Some(Vec::new());

    for transform in transforms {
        let Some(reader_name) = &transform.reader_name else {
            continue;
        };
        let reader_name = reader_name.as_ref();

        let existing_reader = existing_readers.get(reader_name);

        let Some(existing_reader) = existing_reader else {
            let mut serializer = Serializer::from_vec(buffer.take().unwrap_or_default());
            let collection_name = Some(serializer.create_string(&collection.name));
            let reader = {
                let reader_name = Some(serializer.create_string(reader_name));
                let collection_name = Some(serializer.create_string(&transform.source));
                Some(ReaderRecord::create(
                    serializer.buffer_builder(),
                    &ReaderRecordArgs {
                        reader_name,
                        collection_name,
                        generation_id: None,
                    },
                ))
            };
            let request = CreateReaderApiHandler::create_request(
                serializer,
                CreateReaderRequestArgs {
                    collection_name,
                    reader,
                },
            );
            let response_holder = client
                .flatbuffers_call::<CreateReaderApiHandler>(request)
                .await?;
            let _: CreateReaderResponse<'_> = response_holder
                .response()
                .map_err(map_error_response_to_init_collection_error)?;

            buffer = Some(response_holder.into_underlying_vec());
            continue;
        };

        let Some(existing_reader_collection_name) = existing_reader.collection_name() else {
            return Err(InitCollectionsError::Message(format!(
                "reader {reader_name} not pointing to collection"
            )));
        };
        let expected_reader_collection_name = transform.source.as_ref();

        if existing_reader_collection_name != expected_reader_collection_name {
            return Err(InitCollectionsError::Message(format!("reader {reader_name} pointing to collection {existing_reader_collection_name} but expected {expected_reader_collection_name}")));
        }
    }

    Ok(())
}

fn map_error_response_to_init_collection_error(
    err: Option<ErrorResponse<'_>>,
) -> InitCollectionsError {
    let Some(err) = err else {
        return InitCollectionsError::Message(String::from("no response data"));
    };

    InitCollectionsError::Message(format!("{err:?}"))
}
