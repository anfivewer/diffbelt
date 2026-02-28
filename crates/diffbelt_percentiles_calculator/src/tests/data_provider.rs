use core::marker::PhantomData;

use alloc::{collections::btree_map::BTreeMap, vec::Vec};
use hashbrown::HashMap;
use rand::Rng;

use crate::{
    tests::MockPTypes,
    types::{FetchKeysAroundDirection, FetchKeysAroundResponse, PTypes, PercentilesDataProvider},
};

enum PendingAsync<P: PTypes> {

}

pub struct MockDataProvider<P: PTypes> {
    target_records: HashMap<P::TargetKey, P::TargetRecord>,
    source_keys: Vec<P::PercentileKey>,
    pending_asyncs: Vec<(u64, PendingAsync<P>)>,
    phantom: PhantomData<P>,
}

pub struct MockDataProviderOptions<P: PTypes> {
    initial_target_records: HashMap<P::TargetKey, P::TargetRecord>,
    initial_source_keys: Vec<P::PercentileKey>,
}

impl <P: PTypes> MockDataProvider<P> {
    pub fn new(options: MockDataProviderOptions<P>) -> Self {
        Self {
            target_records: options.initial_target_records,
            source_keys: options.initial_source_keys,
            pending_asyncs: Default::default(),
            phantom: Default::default(),
        }
    }

    pub fn resolve_random_async(&mut self, rng: &mut impl Rng) {
        if self.pending_asyncs.is_empty() {
            return;
        }

        let index = rng.next_u64() as usize;
    }
}

impl<P: PTypes> PercentilesDataProvider<P> for MockDataProvider<P> {
    fn get_target_record(&self, key: P::TargetKey) -> Result<u64, P::DataProviderError> {
        todo!()
    }

    fn await_target_record(&self, id: u64) -> Result<P::TargetRecord, P::DataProviderError> {
        todo!()
    }

    fn put_target_record(
        &self,
        key: P::TargetKey,
        record: P::TargetRecord,
    ) -> Result<(), P::DataProviderError> {
        todo!()
    }

    fn fetch_keys_around(
        &self,
        key: P::PercentileKey,
        direction: FetchKeysAroundDirection,
    ) -> Result<u64, P::DataProviderError> {
        todo!()
    }

    fn await_fetch_keys_around(&self, id: u64) -> Result<P::FetchKeysAround, P::DataProviderError> {
        todo!()
    }

    fn insert_key(&self, key: P::PercentileKey) -> Result<u64, P::DataProviderError> {
        todo!()
    }

    fn remove_key(&self, key: P::PercentileKey) -> Result<u64, P::DataProviderError> {
        todo!()
    }

    fn is_async_completed(&self, id: u64) -> Result<bool, P::DataProviderError> {
        todo!()
    }

    fn await_void_async(&self, id: u64) -> Result<(), P::DataProviderError> {
        todo!()
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

pub enum MockDataProviderError {}
