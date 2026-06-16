use core::marker::PhantomData;
use core::ops::Deref;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Condvar, Mutex, OnceLock, RwLock},
    time::Duration,
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

struct Inner<P: PTypes> {
    target_records: HashMap<P::TargetKey, P::TargetRecord>,
    source_keys: Vec<P::PercentileKey>,
    pending_asyncs: Vec<(u64, Arc<PendingAsync<P>>)>,
    pending_asyncs_set: HashSet<u64>,
    pending_asyncs_map: HashMap<u64, Arc<PendingAsync<P>>>,
    async_id_counter: u64,
    insert_key_pending: HashMap<u64, P::PercentileKey>,
    remove_key_pending: HashMap<u64, P::PercentileKey>,
    fetch_keys_around_pending: HashMap<u64, (P::PercentileKey, FetchKeysAroundDirection)>,
}

#[derive(Clone)]
pub struct MockDataProvider<P: PTypes> {
    inner: Arc<RwLock<Inner<P>>>,
    pending_tasks_condvar: Arc<(Mutex<()>, Condvar)>,
    phantom: PhantomData<P>,
}

pub struct MockDataProviderOptions<P: PTypes> {
    pub initial_target_records: HashMap<P::TargetKey, P::TargetRecord>,
    pub initial_source_keys: Vec<P::PercentileKey>,
}

impl<P: PTypes> MockDataProvider<P> {
    pub fn new(options: MockDataProviderOptions<P>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                target_records: options.initial_target_records,
                source_keys: options.initial_source_keys,
                pending_asyncs: Default::default(),
                pending_asyncs_set: Default::default(),
                pending_asyncs_map: Default::default(),
                async_id_counter: 0,
                insert_key_pending: Default::default(),
                remove_key_pending: Default::default(),
                fetch_keys_around_pending: Default::default(),
            })),
            pending_tasks_condvar: Arc::new((Mutex::new(()), Condvar::new())),
            phantom: Default::default(),
        }
    }

    pub fn resolve_random_async(&self, rng: &mut impl Rng) {
        let mut inner = self.inner.write().unwrap();
        if inner.pending_asyncs.is_empty() {
            return;
        }
        let index = u64_to_usize(rng.next_u64()) % inner.pending_asyncs.len();
        let (id, pending) = inner.pending_asyncs.remove(index);
        inner.pending_asyncs_set.remove(&id);
        
        match pending.as_ref() {
            PendingAsync::GetTargetRecord { .. } => {}
            PendingAsync::FetchKeysAround { lock, result } => {
                if let Some((key, direction)) = inner.fetch_keys_around_pending.remove(&id) {
                    let response = self.build_fetch_keys_around_response_locked(&inner, &key, &direction);
                    result.set(response).expect("already initialized");
                }
                lock.set(()).expect("already initialized");
            }
            PendingAsync::InsertKey { lock } => {
                if let Some(key) = inner.insert_key_pending.remove(&id) {
                    let pos = inner.source_keys.binary_search_by(|existing_key| {
                        existing_key.deref().cmp(key.deref())
                    });
                    if let Err(insert_pos) = pos {
                        inner.source_keys.insert(insert_pos, key);
                    }
                }
                lock.set(()).expect("already initialized");
            }
            PendingAsync::RemoveKey { lock } => {
                if let Some(key) = inner.remove_key_pending.remove(&id) {
                    inner.source_keys.retain(|existing_key| {
                        existing_key.deref() != key.deref()
                    });
                }
                lock.set(()).expect("already initialized");
            }
        }
    }

    pub fn resolve_all_asyncs(&self) {
        let mut inner = self.inner.write().unwrap();
        while !inner.pending_asyncs.is_empty() {
            let (id, pending) = inner.pending_asyncs.remove(0);
            inner.pending_asyncs_set.remove(&id);
            
            match pending.as_ref() {
                PendingAsync::GetTargetRecord { .. } => {}
                PendingAsync::FetchKeysAround { lock, result } => {
                    if let Some((key, direction)) = inner.fetch_keys_around_pending.remove(&id) {
                        let response = self.build_fetch_keys_around_response_locked(&inner, &key, &direction);
                        result.set(response).expect("already initialized");
                    }
                    lock.set(()).expect("already initialized");
                }
                PendingAsync::InsertKey { lock } => {
                    if let Some(key) = inner.insert_key_pending.remove(&id) {
                        let pos = inner.source_keys.binary_search_by(|existing_key| {
                            existing_key.deref().cmp(key.deref())
                        });
                        if let Err(insert_pos) = pos {
                            inner.source_keys.insert(insert_pos, key);
                        }
                    }
                    lock.set(()).expect("already initialized");
                }
                PendingAsync::RemoveKey { lock } => {
                    if let Some(key) = inner.remove_key_pending.remove(&id) {
                        inner.source_keys.retain(|existing_key| {
                            existing_key.deref() != key.deref()
                        });
                    }
                    lock.set(()).expect("already initialized");
                }
            }
        }
    }

    pub fn pending_asyncs_count(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.pending_asyncs.len()
    }

    fn build_fetch_keys_around_response(
        &self,
        key: &P::PercentileKey,
        direction: &FetchKeysAroundDirection,
    ) -> MockFetchAroundResponse {
        let inner = self.inner.read().unwrap();
        self.build_fetch_keys_around_response_locked(&inner, key, direction)
    }

    fn build_fetch_keys_around_response_locked(
        &self,
        inner: &Inner<P>,
        key: &P::PercentileKey,
        direction: &FetchKeysAroundDirection,
    ) -> MockFetchAroundResponse {
        let center_key: Arc<[u8]> = Arc::from(key.deref());

        let found_position =
            inner.source_keys
                .binary_search_by(|existing_key| existing_key.deref().cmp(key.deref()));

        let (left_keys, right_keys) = match found_position {
            Ok(center_index) => {
                let left = match direction {
                    FetchKeysAroundDirection::Left | FetchKeysAroundDirection::Both => {
                        let mut reversed_left: Vec<Arc<[u8]>> = inner.source_keys[..center_index]
                            .iter()
                            .map(|k| Arc::from(k.deref()))
                            .collect();
                        reversed_left.reverse();
                        reversed_left
                    }
                    _ => vec![],
                };
                let right = match direction {
                    FetchKeysAroundDirection::Right | FetchKeysAroundDirection::Both => {
                        inner.source_keys[center_index + 1..]
                            .iter()
                            .map(|k| Arc::from(k.deref()))
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

    fn insert_pending_async(&self, pending: PendingAsync<P>) -> u64 {
        let mut inner = self.inner.write().unwrap();
        inner.async_id_counter += 1;
        let id = inner.async_id_counter;
        let pending = Arc::new(pending);

        inner.pending_asyncs.push((id, pending.clone()));
        inner.pending_asyncs_set.insert(id);
        inner.pending_asyncs_map.insert(id, pending.clone());
        
        drop(inner);
        let (lock, condvar) = &*self.pending_tasks_condvar;
        condvar.notify_all();

        id
    }

    pub fn await_some_pending_tasks(&self, timeout: Duration) -> bool {
        let (lock, condvar) = &*self.pending_tasks_condvar;
        let guard = lock.lock().unwrap();
        let _result = condvar.wait_timeout(guard, timeout).unwrap();
        
        let inner = self.inner.read().unwrap();
        !inner.pending_asyncs.is_empty()
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
        let pending = {
            let inner = self.inner.read().unwrap();
            inner.pending_asyncs_map.get(&id).cloned()
        }.ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

        let Some((key, lock)) = pending.as_get_target_record() else {
            return Err(MockDataProviderError::MessageStatic(
                "not target record pending",
            ));
        };

        lock.wait();

        let target_record = {
            let inner = self.inner.read().unwrap();
            inner.target_records.get(key).cloned()
        };

        {
            let mut inner = self.inner.write().unwrap();
            inner.pending_asyncs_map.remove(&id);
        }

        Ok(target_record)
    }

    fn put_target_record(
        &mut self,
        key: <MockPTypes as PTypes>::TargetKey,
        record: <MockPTypes as PTypes>::TargetRecord,
    ) -> Result<(), <MockPTypes as PTypes>::DataProviderError> {
        let mut inner = self.inner.write().unwrap();
        inner.target_records.insert(key, record);
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
        {
            let mut inner = self.inner.write().unwrap();
            inner.fetch_keys_around_pending.insert(id, (key, direction));
        }
        Ok(id)
    }

    fn await_fetch_keys_around(
        &mut self,
        id: u64,
    ) -> Result<<MockPTypes as PTypes>::FetchKeysAround, <MockPTypes as PTypes>::DataProviderError>
    {
        let pending = {
            let inner = self.inner.read().unwrap();
            inner.pending_asyncs_map.get(&id).cloned()
        }.ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

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

        {
            let mut inner = self.inner.write().unwrap();
            inner.pending_asyncs_map.remove(&id);
        }

        Ok(response)
    }

    fn insert_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        let id = self.insert_pending_async(PendingAsync::InsertKey {
            lock: OnceLock::new(),
        });
        {
            let mut inner = self.inner.write().unwrap();
            inner.insert_key_pending.insert(id, key);
        }
        Ok(id)
    }

    fn remove_key(
        &mut self,
        key: <MockPTypes as PTypes>::PercentileKey,
    ) -> Result<u64, <MockPTypes as PTypes>::DataProviderError> {
        let id = self.insert_pending_async(PendingAsync::RemoveKey {
            lock: OnceLock::new(),
        });
        {
            let mut inner = self.inner.write().unwrap();
            inner.remove_key_pending.insert(id, key);
        }
        Ok(id)
    }

    fn is_async_completed(
        &self,
        id: u64,
    ) -> Result<bool, <MockPTypes as PTypes>::DataProviderError> {
        let inner = self.inner.read().unwrap();
        if !inner.pending_asyncs_map.contains_key(&id) {
            return Err(MockDataProviderError::Message(format!("No async {id}")));
        }

        Ok(inner.pending_asyncs_set.contains(&id))
    }

    fn await_void_async(
        &mut self,
        id: u64,
    ) -> Result<(), <MockPTypes as PTypes>::DataProviderError> {
        let pending = {
            let inner = self.inner.read().unwrap();
            inner.pending_asyncs_map.get(&id).cloned()
        }.ok_or_else(|| MockDataProviderError::Message(format!("No pending {id}")))?;

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

        {
            let mut inner = self.inner.write().unwrap();
            inner.pending_asyncs_map.remove(&id);
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct MockFetchAroundResponse {
    center_key: Arc<[u8]>,
    left_keys: Vec<Arc<[u8]>>,
    right_keys: Vec<Arc<[u8]>>,
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
    pub fn get_all_target_keys(&self) -> Vec<<MockPTypes as PTypes>::TargetKey> {
        let inner = self.inner.read().unwrap();
        inner.target_records.keys().cloned().collect()
    }

    pub fn get_target_record(
        &self,
        key: &<MockPTypes as PTypes>::TargetKey,
    ) -> Option<<MockPTypes as PTypes>::TargetRecord> {
        let inner = self.inner.read().unwrap();
        inner.target_records.get(key).cloned()
    }
}

pub enum MockDataProviderError {
    Message(String),
    MessageStatic(&'static str),
}
