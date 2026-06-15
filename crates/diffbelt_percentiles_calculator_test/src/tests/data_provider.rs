use core::marker::PhantomData;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, OnceLock},
};

use diffbelt_util_no_std::cast::u64_to_usize;
use enum_as_inner::EnumAsInner;
use rand::Rng;

use diffbelt_percentiles_calculator::types::{
    FetchKeysAroundDirection, FetchKeysAroundResponse, PTypes, PercentilesDataProvider,
};

use crate::tests::MockPTypes;

#[derive(EnumAsInner)]
enum PendingAsync<P: PTypes> {
    GetTargetRecord {
        key: P::TargetKey,
        lock: OnceLock<()>,
    },
}

pub struct MockDataProvider<P: PTypes> {
    target_records: HashMap<P::TargetKey, P::TargetRecord>,
    source_keys: Vec<P::PercentileKey>,
    pending_asyncs: Vec<(u64, Arc<PendingAsync<P>>)>,
    pending_asyncs_set: HashSet<u64>,
    pending_asyncs_map: HashMap<u64, Arc<PendingAsync<P>>>,
    async_id_counter: u64,
    phantom: PhantomData<P>,
}

pub struct MockDataProviderOptions<P: PTypes> {
    initial_target_records: HashMap<P::TargetKey, P::TargetRecord>,
    initial_source_keys: Vec<P::PercentileKey>,
}

impl<P: PTypes> MockDataProvider<P> {
    pub fn new(options: MockDataProviderOptions<P>) -> Self {
        Self {
            target_records: options.initial_target_records,
            source_keys: options.initial_source_keys,
            pending_asyncs: Default::default(),
            pending_asyncs_set: Default::default(),
            pending_asyncs_map: Default::default(),
            async_id_counter: 0,
            phantom: Default::default(),
        }
    }

    pub fn resolve_random_async(&mut self, rng: &mut impl Rng) {
        if self.pending_asyncs.is_empty() {
            return;
        }

        let index = u64_to_usize(rng.next_u64()) % self.pending_asyncs.len();

        let (id, pending) = self.pending_asyncs.remove(index);

        let lock = match pending.as_ref() {
            PendingAsync::GetTargetRecord { lock, .. } => lock,
        };

        self.pending_asyncs_set.remove(&id);

        lock.set(()).expect("already initialized");
    }

    fn insert_pending_async(&mut self, pending: PendingAsync<P>) -> u64 {
        self.async_id_counter += 1;
        let id = self.async_id_counter;
        let pending = Arc::new(pending);

        self.pending_asyncs.push((id, pending.clone()));
        self.pending_asyncs_set.insert(id);
        self.pending_asyncs_map.insert(id, pending.clone());

        id
    }
}

impl PercentilesDataProvider<MockPTypes> for MockDataProvider<MockPTypes> {
    fn get_target_record(
        &mut self,
        key: <MockPTypes as PTypes>::TargetKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        Ok(self.insert_pending_async(PendingAsync::GetTargetRecord {
            key,
            lock: OnceLock::new(),
        }))
    }

    fn await_target_record(
        &mut self,
        id: u64,
    ) -> Result<
        Option<<MockPTypes as PTypes>::TargetRecord>,
        <MockPTypes as PTypes>::DataProviderError,
    > {
        let pending = self
            .pending_asyncs_map
            .get(&id)
            .ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

        let Some((key, lock)) = pending.as_get_target_record() else {
            return Err(MockDataProviderError::MessageStatic(
                "not target record pending",
            ));
        };

        lock.wait();

        let target_record = self.target_records.get(key).map(|x| x.clone());

        self.pending_asyncs_map.remove(&id);

        Ok(target_record)
    }

    fn put_target_record(
        &mut self,
        key: <MockPTypes as PTypes>::TargetKey,
        record: <MockPTypes as PTypes>::TargetRecord,
    ) -> Result<(), <MockPTypes as PTypes>::DataProviderError> {
        todo!()
    }

    fn fetch_keys_around(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
        direction: FetchKeysAroundDirection,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        todo!()
    }

    fn await_fetch_keys_around(
        &mut self,
        id: u64,
    ) -> Result<<MockPTypes as PTypes>::FetchKeysAround, <MockPTypes as PTypes>::DataProviderError>
    {
        todo!()
    }

    fn insert_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        todo!()
    }

    fn remove_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        todo!()
    }

    fn is_async_completed(
        &self,
        id: u64,
    ) -> Result<bool, <MockPTypes as PTypes>::DataProviderError> {
        if !self.pending_asyncs_map.contains_key(&id) {
            return Err(MockDataProviderError::Message(format!("No async {id}")));
        }

        Ok(self.pending_asyncs_set.contains(&id))
    }

    fn await_void_async(
        &mut self,
        id: u64,
    ) -> Result<(), <MockPTypes as PTypes>::DataProviderError> {
        let pending = self
            .pending_asyncs_map
            .get(&id)
            .ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

        let lock = match pending.as_ref() {
            PendingAsync::GetTargetRecord { lock, .. } => lock,
        };

        lock.wait();

        self.pending_asyncs_map.remove(&id);

        Ok(())
    }
}

pub struct MockFetchAroundResponse;

impl FetchKeysAroundResponse<MockPTypes> for MockFetchAroundResponse {
    fn center(&self) -> <MockPTypes as PTypes>::PercentileKey {
        todo!()
    }

    fn direction() -> FetchKeysAroundDirection {
        todo!()
    }

    fn left(&self) -> &[<MockPTypes as PTypes>::PercentileKey] {
        todo!()
    }

    fn right(&self) -> &[<MockPTypes as PTypes>::PercentileKey] {
        todo!()
    }
}

pub enum MockDataProviderError {
    Message(String),
    MessageStatic(&'static str),
}
