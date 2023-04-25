use crate::common::generation_id::GenerationIdSource;
use crate::common::{GenerationId, OwnedGenerationId, PhantomId};
use crate::database::cursors::storage::{CursorPublicId, CursorType};
use crate::raw_db::diff_collection_records::DiffCursorState;

pub struct DiffCursor {
    pub public_id: CursorPublicId,
    pub from_generation_id: GenerationIdSource,
    pub to_generation_id: OwnedGenerationId,
    pub omit_intermediate_values: bool,
    pub raw_db_cursor_state: Option<DiffCursorState>,
}

pub struct AddDiffCursorData {
    pub from_generation_id: Option<OwnedGenerationId>,
    // Result can be returned with generation_id <= to_generation_id_loose
    pub to_generation_id: OwnedGenerationId,
    pub omit_intermediate_values: bool,
    pub raw_db_cursor_state: Option<DiffCursorState>,
}

pub struct AddDiffCursorContinuationData {
    pub next_diff_state: Option<DiffCursorState>,
}

#[derive(Copy, Clone)]
pub struct DiffCursorType;

impl CursorType for DiffCursorType {
    type Data = DiffCursor;
    type AddData = AddDiffCursorData;
    type AddContinuationData = AddDiffCursorContinuationData;

    fn public_id_from_data(data: &Self::Data) -> CursorPublicId {
        data.public_id
    }

    fn phantom_id_from_data(_: &Self::Data) -> Option<PhantomId<'_>> {
        None
    }

    fn from_generation_id_from_add_data(data: &Self::AddData) -> Option<GenerationId<'_>> {
        data.from_generation_id.as_ref().map(|x| x.as_ref())
    }

    fn to_generation_id_from_add_data(data: &Self::AddData) -> GenerationId<'_> {
        data.to_generation_id.as_ref()
    }

    fn data_from_add_data(data: Self::AddData, public_id: CursorPublicId) -> Self::Data {
        let AddDiffCursorData {
            from_generation_id,
            to_generation_id,
            omit_intermediate_values,
            raw_db_cursor_state,
        } = data;

        DiffCursor {
            public_id,
            from_generation_id: GenerationIdSource::Value(from_generation_id),
            to_generation_id,
            omit_intermediate_values,
            raw_db_cursor_state,
        }
    }

    fn replace_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
    ) -> Self::Data {
        let AddDiffCursorContinuationData { next_diff_state } = continuation_data;

        let DiffCursor {
            public_id,
            from_generation_id,
            to_generation_id,
            omit_intermediate_values,
            raw_db_cursor_state: _,
        } = data;

        DiffCursor {
            public_id: public_id.clone(),
            from_generation_id: from_generation_id.clone(),
            to_generation_id: to_generation_id.clone(),
            omit_intermediate_values: *omit_intermediate_values,
            raw_db_cursor_state: next_diff_state,
        }
    }

    fn new_data_from_continuation(
        continuation_data: Self::AddContinuationData,
        data: &Self::Data,
        public_id: CursorPublicId,
    ) -> Self::Data {
        let AddDiffCursorContinuationData { next_diff_state } = continuation_data;

        let DiffCursor {
            public_id: _,
            from_generation_id,
            to_generation_id,
            omit_intermediate_values,
            raw_db_cursor_state: _,
        } = data;

        DiffCursor {
            public_id,
            from_generation_id: from_generation_id.clone(),
            to_generation_id: to_generation_id.clone(),
            omit_intermediate_values: *omit_intermediate_values,
            raw_db_cursor_state: next_diff_state,
        }
    }
}
