use crate::common::GenerationId;
use crate::database::generations::collection::GenerationIdNextGenerationIdPair;
use crate::util::async_lock::AsyncLockInstance;

pub struct NextGenerationIdLockData {
    pub need_schedule_next_generation: bool,
}

impl NextGenerationIdLockData {
    pub fn new() -> Self {
        Self {
            need_schedule_next_generation: false,
        }
    }
}

pub struct GenerationIdLock {
    pub async_lock_instance:
        AsyncLockInstance<GenerationIdNextGenerationIdPair, NextGenerationIdLockData>,
}

impl GenerationIdLock {
    pub fn set_need_schedule_next_generation(&mut self) {
        let data = self.async_lock_instance.data_mut();
        data.need_schedule_next_generation = true;
    }

    pub fn generation_id(&self) -> GenerationId<'_> {
        self.async_lock_instance.value().generation_id.as_ref()
    }
}
