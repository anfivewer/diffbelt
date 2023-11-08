use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::common::generation_id::EncodedGenerationIdJsonData;

pub enum State {
    Uninitialized,
    AwaitingDiff,
    AwaitingGenerationStart {
        diff: DiffCollectionResponseJsonData,
    },
    Processing(ProcessingState),
    Invalid,
}

pub struct ProcessingState {
    pub cursor_id: Option<Box<str>>,
    pub to_generation_id: EncodedGenerationIdJsonData,
}
