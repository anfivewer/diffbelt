use core::marker::PhantomData;
use core::ops::Deref;
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
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
    FetchKeysAround {
        lock: OnceLock<()>,
        result: OnceLock<MockFetchAroundResponse>,
    },
    InsertKey {
        lock: OnceLock<()>,
    },
    RemoveKey {
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
    insert_key_pending: HashMap<u64, P::PercentileKey>,
    remove_key_pending: HashMap<u64, P::PercentileKey>,
    fetch_keys_around_pending: HashMap<u64, (P::PercentileKey, FetchKeysAroundDirection)>,
    phantom: PhantomData<P>,
}

pub struct MockDataProviderOptions<P: PTypes> {
    pub initial_target_records: HashMap<P::TargetKey, P::TargetRecord>,
    pub initial_source_keys: Vec<P::PercentileKey>,
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
            insert_key_pending: Default::default(),
            remove_key_pending: Default::default(),
            fetch_keys_around_pending: Default::default(),
            phantom: Default::default(),
        }
    }

    fn process_pending(&mut self, id: u64, pending: Arc<PendingAsync<P>>) {
        match pending.as_ref() {
            PendingAsync::GetTargetRecord { .. } => {}
            PendingAsync::FetchKeysAround { lock, result } => {
                if let Some((key, direction)) = self.fetch_keys_around_pending.remove(&id) {
                    let response = self.build_fetch_keys_around_response(&key, &direction);
                    result.set(response).expect("already initialized");
                }
                lock.set(()).expect("already initialized");
            }
            PendingAsync::InsertKey { lock } => {
                if let Some(key) = self.insert_key_pending.remove(&id) {
                    let pos =
                        self.source_keys
                            .binary_search_by(|existing_key| {
                                existing_key.deref().cmp(key.deref())
                            });
                    if let Err(insert_pos) = pos {
                        self.source_keys.insert(insert_pos, key);
                    }
                }
                lock.set(()).expect("already initialized");
            }
            PendingAsync::RemoveKey { lock } => {
                if let Some(key) = self.remove_key_pending.remove(&id) {
                    self.source_keys.retain(|existing_key| {
                        existing_key.deref() != key.deref()
                    });
                }
                lock.set(()).expect("already initialized");
            }
        }
    }

    pub fn resolve_random_async(&mut self, rng: &mut impl Rng) {
        if self.pending_asyncs.is_empty() {
            return;
        }
        let index = u64_to_usize(rng.next_u64()) % self.pending_asyncs.len();
        let (id, pending) = self.pending_asyncs.remove(index);
        self.pending_asyncs_set.remove(&id);
        self.process_pending(id, pending);
    }

    pub fn resolve_all_asyncs(&mut self) {
        while !self.pending_asyncs.is_empty() {
            let (id, pending) = self.pending_asyncs.remove(0);
            self.pending_asyncs_set.remove(&id);
            self.process_pending(id, pending);
        }
    }

    pub fn pending_asyncs_count(&self) -> usize {
        self.pending_asyncs.len()
    }

    fn build_fetch_keys_around_response(
        &self,
        key: &P::PercentileKey,
        direction: &FetchKeysAroundDirection,
    ) -> MockFetchAroundResponse {
        let center_key: Rc<[u8]> = Rc::from(key.deref());

        let found_position =
            self.source_keys
                .binary_search_by(|existing_key| existing_key.deref().cmp(key.deref()));

        let (left_keys, right_keys) = match found_position {
            Ok(center_index) => {
                let left = match direction {
                    FetchKeysAroundDirection::Left | FetchKeysAroundDirection::Both => {
                        let mut reversed_left: Vec<Rc<[u8]>> = self.source_keys[..center_index]
                            .iter()
                            .map(|k| Rc::from(k.deref()))
                            .collect();
                        reversed_left.reverse();
                        reversed_left
                    }
                    _ => vec![],
                };
                let right = match direction {
                    FetchKeysAroundDirection::Right | FetchKeysAroundDirection::Both => {
                        self.source_keys[center_index + 1..]
                            .iter()
                            .map(|k| Rc::from(k.deref()))
                            .collect()
                    }
                    _ => vec![],
                };
                (left, right)
            }
            Err(_) => (vec![], vec![]),
        };

        MockFetchAroundResponse {
            center_key,
            left_keys,
            right_keys,
        }
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
        self.target_records.insert(key, record);
        Ok(())
    }

    fn fetch_keys_around(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
        direction: FetchKeysAroundDirection,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        let id = self.insert_pending_async(PendingAsync::FetchKeysAround {
            lock: OnceLock::new(),
            result: OnceLock::new(),
        });
        self.fetch_keys_around_pending.insert(id, (key, direction));
        Ok(id)
    }

    fn await_fetch_keys_around(
        &mut self,
        id: u64,
    ) -> Result<<MockPTypes as PTypes>::FetchKeysAround, <MockPTypes as PTypes>::DataProviderError>
    {
        let pending = self
            .pending_asyncs_map
            .get(&id)
            .ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

        let Some((lock, result)) = pending.as_fetch_keys_around() else {
            return Err(MockDataProviderError::MessageStatic(
                "not fetch keys around pending",
            ));
        };

        lock.wait();

        let response = result
            .get()
            .expect("fetch keys around result not set")
            .clone();

        self.pending_asyncs_map.remove(&id);

        Ok(response)
    }

    fn insert_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        let id = self.insert_pending_async(PendingAsync::InsertKey {
            lock: OnceLock::new(),
        });
        self.insert_key_pending.insert(id, key);
        Ok(id)
    }

    fn remove_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        let id = self.insert_pending_async(PendingAsync::RemoveKey {
            lock: OnceLock::new(),
        });
        self.remove_key_pending.insert(id, key);
        Ok(id)
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
            PendingAsync::InsertKey { lock } => lock,
            PendingAsync::RemoveKey { lock } => lock,
            PendingAsync::GetTargetRecord { .. } => {
                return Err(MockDataProviderError::MessageStatic(
                    "use await_target_record for get_target_record async",
                ));
            }
            PendingAsync::FetchKeysAround { .. } => {
                return Err(MockDataProviderError::MessageStatic(
                    "use await_fetch_keys_around for fetch_keys_around async",
                ));
            }
        };

        lock.wait();

        self.pending_asyncs_map.remove(&id);

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MockFetchAroundResponse {
    center_key: Rc<[u8]>,
    left_keys: Vec<Rc<[u8]>>,
    right_keys: Vec<Rc<[u8]>>,
}

impl FetchKeysAroundResponse<MockPTypes> for MockFetchAroundResponse {
    fn center(&self) -> <MockPTypes as PTypes>::PercentileKey {
        self.center_key.clone()
    }

    fn direction() -> FetchKeysAroundDirection {
        FetchKeysAroundDirection::Both
    }

    fn left(&self) -> &[<MockPTypes as PTypes>::PercentileKey] {
        &self.left_keys
    }

    fn right(&self) -> &[<MockPTypes as PTypes>::PercentileKey] {
        &self.right_keys
    }
}

impl MockDataProvider<MockPTypes> {
    pub fn get_target_record_direct(
        &self,
        key: &<MockPTypes as PTypes>::TargetKey,
    ) -> Option<&<MockPTypes as PTypes>::TargetRecord> {
        self.target_records.get(key)
    }
}

pub enum MockDataProviderError {
    Message(String),
    MessageStatic(&'static str),
}
