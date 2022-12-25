use crate::collection::methods::commit_generation::CommitGenerationOptions;
use crate::collection::methods::start_generation::StartGenerationOptions;
use crate::collection::Collection;
use crate::common::GenerationId;
use std::future::Future;

pub async fn wrap_generation(
    collection: &Collection,
    generation_id: GenerationId<'_>,
    fut: impl Future<Output = ()>,
) {
    collection
        .start_generation(StartGenerationOptions {
            generation_id: generation_id.to_owned(),
            abort_outdated: false,
        })
        .await
        .unwrap();

    fut.await;

    collection
        .commit_generation(CommitGenerationOptions {
            generation_id: generation_id.to_owned(),
        })
        .await
        .unwrap();
}
