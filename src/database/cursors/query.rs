use crate::common::{GenerationId, OwnedGenerationId, OwnedPhantomId, PhantomId};
use crate::database::cursors::storage::{CursorPublicId, CursorType};
use crate::raw_db::query_collection_records::LastAndNextRecordKey;

pub struct QueryCursor {
    pub public_id: CursorPublicId,
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct AddQueryCursorData {
    pub generation_id: OwnedGenerationId,
    pub phantom_id: Option<OwnedPhantomId>,
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

pub struct AddQueryCursorContinuationData {
    pub last_and_next_record_key: Option<LastAndNextRecordKey>,
}

#[derive(Copy, Clone)]
pub struct QueryCursorType;

impl CursorType for QueryCursorType {
    type Data = QueryCursor;
    type AddData = AddQueryCursorData;
    type AddContinuationData = AddQueryCursorContinuationData;

    fn public_id_from_data(data: &Self::Data) -> CursorPublicId {
        data.public_id
    }

    fn generation_id_from_data(data: &Self::Data) -> GenerationId<'_> {
        data.generation_id.as_ref()
    }

    fn phantom_id_from_data(data: &Self::Data) -> Option<PhantomId<'_>> {
        data.phantom_id.as_ref().map(|x| x.as_ref())
    }

    fn generation_id_from_add_data(data: &Self::AddData) -> GenerationId<'_> {
        data.generation_id.as_ref()
    }

    fn data_from_add_data(data: Self::AddData, public_id: CursorPublicId) -> Self::Data {
        QueryCursor {
            public_id,
            generation_id: data.generation_id,
            phantom_id: data.phantom_id,
            last_and_next_record_key: data.last_and_next_record_key,
        }
    }

    fn replace_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
    ) -> Self::Data {
        let AddQueryCursorContinuationData {
            last_and_next_record_key,
        } = continuation_data;

        let QueryCursor {
            public_id,
            generation_id,
            phantom_id,
            last_and_next_record_key: _,
        } = data;

        QueryCursor {
            public_id: public_id.clone(),
            generation_id: generation_id.clone(),
            phantom_id: phantom_id.clone(),
            last_and_next_record_key,
        }
    }

    fn new_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
        public_id: CursorPublicId,
    ) -> Self::Data {
        let AddQueryCursorContinuationData {
            last_and_next_record_key,
        } = continuation_data;

        let QueryCursor {
            public_id: _,
            generation_id,
            phantom_id,
            last_and_next_record_key: _,
        } = data;

        QueryCursor {
            public_id,
            generation_id: generation_id.clone(),
            phantom_id: phantom_id.clone(),
            last_and_next_record_key,
        }
    }
}
