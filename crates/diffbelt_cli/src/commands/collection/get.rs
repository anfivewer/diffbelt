use crate::commands::collection::Collection;
use crate::format::generation_id::format_generation_id;
use crate::state::CliState;
use crate::CommandResult;
use diffbelt_types::collection::get::GetCollectionResponseJsonData;
use std::sync::Arc;

pub async fn get_collection_command(command: &Collection, state: Arc<CliState>) -> CommandResult {
    let response = state.client.get_collection(&command.name).await.unwrap();

    let GetCollectionResponseJsonData {
        is_manual,
        generation_id,
        next_generation_id,
    } = response;

    println!("Name: {}", &command.name);
    println!("Is manual: {}", if is_manual { "Yes" } else { "No" });
    println!(
        "GenerationId: {}",
        format_generation_id(&generation_id.unwrap())
    );

    let next_generation_id =
        next_generation_id.and_then(|id| id.map(|id| format_generation_id(&id)));
    let next_generation_id = next_generation_id
        .as_ref()
        .map(|id| id.as_str())
        .unwrap_or("---");

    println!("Next generationId: {}", next_generation_id);

    Ok(())
}
