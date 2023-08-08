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

pub struct NextGenerationIdLock {
    pub async_lock_instance:
        AsyncLockInstance<GenerationIdNextGenerationIdPair, NextGenerationIdLockData>,
}

impl NextGenerationIdLock {
    pub fn set_need_schedule_next_generation(&mut self) {
        let data = self.async_lock_instance.data_mut();
        data.need_schedule_next_generation = true;
    }
}
