use crate::Collection;
use diffbelt_http_client::client::DiffbeltClient;
use diffbelt_http_client::errors::DiffbeltClientError;
use diffbelt_protos::protos::api::collection::CreateCollectionRequestArgs;
use diffbelt_protos::protos::handlers::{ApiHandler, CreateCollectionApiHandler};
use diffbelt_protos::protos::impls::RequestProto;
use diffbelt_protos::Serializer;
use std::collections::HashMap;
use thiserror::Error;
use crate::transforms::Transform;

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
    }

    Ok(())
}

pub async fn init_collection_readers(collection: &Collection, transforms: &[Transform]) {
    //
}