use std::mem;

use rocksdb::{DBIteratorWithThreadMode, DBPinnableSlice, Direction, IteratorMode, DB};

use crate::collection::util::record_key::{
    OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey, RecordKey,
};
use crate::common::{
    CollectionKey, CollectionValue, GenerationId, IsByteArray, OwnedGenerationId, OwnedPhantomId,
    PhantomId,
};
use crate::raw_db::RawDbError;

pub struct QueryKeyValue;
pub struct QueryKeysOnly;

pub struct QueryDirectionBackward;
pub struct QueryDirectionForward;

pub struct ContinuationState {
    pub last_candidate_key: OwnedRecordKey,
    pub next_iterator_key: OwnedRecordKey,
}

pub struct QueryOptions<'a, K: QueryKind, D: QueryDirection> {
    pub kind: K,
    pub direction: D,
    pub start_key: Option<CollectionKey<'a>>,
    pub generation_id: GenerationId<'a>,
    pub phantom_id: Option<PhantomId<'a>>,
    pub gc_phantom_id: Option<DBPinnableSlice<'a>>,
    pub continuation_state: Option<ContinuationState>,
    pub records_to_view_limit: usize,
}

pub trait QueryKind {
    type Item;
    type Value;

    fn is_value_needed() -> bool;
    fn empty_kv_record() -> IterationKvRecord<Self::Value>;
    fn empty_value() -> Self::Value;
    fn value_from_vec(value: Vec<u8>) -> Self::Value;
    fn value_from_box(value: Box<[u8]>) -> Self::Value;
    fn make_item(record: IterationKvRecord<Self::Value>) -> Option<Self::Item>;
}

impl QueryKind for QueryKeyValue {
    type Item = IterationKvRecord<Self::Value>;
    type Value = Box<[u8]>;

    fn is_value_needed() -> bool {
        true
    }
    fn empty_kv_record() -> IterationKvRecord<Self::Value> {
        IterationKvRecord {
            key: OwnedParsedRecordKey::empty(),
            value: Self::empty_value(),
            is_visible: false,
        }
    }
    fn empty_value() -> Self::Value {
        Box::new([])
    }
    fn value_from_vec(value: Vec<u8>) -> Self::Value {
        value.into()
    }
    fn value_from_box(value: Box<[u8]>) -> Self::Value {
        value
    }
    fn make_item(record: IterationKvRecord<Self::Value>) -> Option<Self::Item> {
        let value = CollectionValue::from_slice(&record.value);
        let is_empty = value.is_empty();

        if is_empty {
            None
        } else {
            Some(record)
        }
    }
}

impl QueryKind for QueryKeysOnly {
    type Item = OwnedParsedRecordKey;
    type Value = bool;

    fn is_value_needed() -> bool {
        false
    }
    fn empty_kv_record() -> IterationKvRecord<Self::Value> {
        IterationKvRecord {
            key: OwnedParsedRecordKey::empty(),
            value: Self::empty_value(),
            is_visible: false,
        }
    }
    fn empty_value() -> Self::Value {
        false
    }
    fn value_from_vec(value: Vec<u8>) -> Self::Value {
        let value = CollectionValue::from_slice(&value);
        !value.is_empty()
    }
    fn value_from_box(value: Box<[u8]>) -> Self::Value {
        let value = CollectionValue::from_slice(&value);
        !value.is_empty()
    }
    fn make_item(record: IterationKvRecord<Self::Value>) -> Option<Self::Item> {
        if record.value {
            Some(record.key)
        } else {
            None
        }
    }
}

pub trait QueryDirection {
    fn is_forward() -> bool;
    fn get_initial_record_generation_id() -> GenerationId<'static>;
    fn get_initial_record_phantom_id() -> PhantomId<'static>;
    fn get_default_iterator_mode() -> IteratorMode<'static>;
    fn get_direction() -> Direction;
    fn is_suitable_generation_id(
        generation_id: GenerationId<'_>,
        last_generation_id: GenerationId<'_>,
        next_generation_id: GenerationId<'_>,
    ) -> bool;
}

impl QueryDirection for QueryDirectionBackward {
    fn is_forward() -> bool {
        false
    }
    fn get_initial_record_generation_id() -> GenerationId<'static> {
        GenerationId::max_value()
    }
    fn get_initial_record_phantom_id() -> PhantomId<'static> {
        PhantomId::max_value()
    }
    fn get_default_iterator_mode() -> IteratorMode<'static> {
        IteratorMode::End
    }
    fn get_direction() -> Direction {
        Direction::Reverse
    }
    fn is_suitable_generation_id(
        generation_id: GenerationId<'_>,
        last_generation_id: GenerationId<'_>,
        next_generation_id: GenerationId<'_>,
    ) -> bool {
        if last_generation_id <= generation_id {
            return false;
        }

        next_generation_id <= generation_id
    }
}

impl QueryDirection for QueryDirectionForward {
    fn is_forward() -> bool {
        true
    }
    fn get_initial_record_generation_id() -> GenerationId<'static> {
        GenerationId::empty()
    }
    fn get_initial_record_phantom_id() -> PhantomId<'static> {
        PhantomId::empty()
    }
    fn get_default_iterator_mode() -> IteratorMode<'static> {
        IteratorMode::Start
    }
    fn get_direction() -> Direction {
        Direction::Forward
    }
    fn is_suitable_generation_id(
        generation_id: GenerationId<'_>,
        _: GenerationId<'_>,
        next_generation_id: GenerationId<'_>,
    ) -> bool {
        if next_generation_id <= generation_id {
            return true;
        }

        false
    }
}

pub struct IterationKvRecord<T> {
    pub key: OwnedParsedRecordKey,
    pub value: T,
    pub is_visible: bool,
}

pub struct QueryState<'a, K: QueryKind, D: QueryDirection> {
    is_empty: bool,
    // Those fields are zero-sized, used only for proper typings and not to forget about them
    #[allow(dead_code)]
    kind: K,
    #[allow(dead_code)]
    direction: D,
    generation_id: OwnedGenerationId,
    phantom_id: Option<OwnedPhantomId>,
    gc_phantom_id: Option<DBPinnableSlice<'a>>,
    pub records_to_delete: Vec<OwnedRecordKey>,
    records_seen: usize,
    records_to_view_limit: usize,
    iterator: Option<DBIteratorWithThreadMode<'a, DB>>,
    last_record: IterationKvRecord<K::Value>,
    next_record: Option<IterationKvRecord<K::Value>>,
}

impl<'a, K: QueryKind, D: QueryDirection> QueryState<'a, K, D> {
    pub fn new(db: &'a DB, options: QueryOptions<'a, K, D>) -> Result<Self, RawDbError> {
        let QueryOptions {
            kind,
            direction,
            start_key,
            generation_id,
            phantom_id,
            gc_phantom_id,
            continuation_state,
            records_to_view_limit,
        } = options;

        let (last_candidate_key, next_iterator_key) = match continuation_state {
            Some(ContinuationState {
                last_candidate_key,
                next_iterator_key,
            }) => (Some(last_candidate_key), Some(next_iterator_key)),
            None => (None, None),
        };

        let mut iterator = create_iterator::<D>(
            db,
            start_key,
            generation_id,
            phantom_id,
            next_iterator_key.as_ref().map(|x| x.as_ref()),
        )?;

        let gc_phantom_id_ref = gc_phantom_id
            .as_ref()
            .map(|x| PhantomId::new_unchecked(x.as_ref()));

        let initialization_result = initialize_last_and_next::<K>(
            db,
            &mut iterator,
            last_candidate_key,
            gc_phantom_id_ref,
        )?;

        match initialization_result {
            InitializationResult::End => Ok(Self {
                is_empty: true,
                kind,
                direction,
                generation_id: generation_id.to_owned(),
                phantom_id: None,
                gc_phantom_id: None,
                records_to_delete: Vec::new(),
                records_seen: 0,
                records_to_view_limit,
                iterator: None,
                last_record: K::empty_kv_record(),
                next_record: None,
            }),
            InitializationResult::Full {
                last_record,
                next_record,
            } => Ok(Self {
                is_empty: false,
                kind,
                direction,
                generation_id: generation_id.to_owned(),
                phantom_id: phantom_id.map(|x| x.to_owned()),
                gc_phantom_id,
                records_to_delete: Vec::new(),
                records_seen: 0,
                records_to_view_limit,
                iterator: Some(iterator),
                last_record,
                next_record,
            }),
        }
    }

    pub fn is_end(&self) -> bool {
        self.is_empty
    }

    pub fn into_continuation(self) -> Option<ContinuationState> {
        if self.is_end() {
            return None;
        }

        let next_record = self.next_record.unwrap();

        Some(ContinuationState {
            last_candidate_key: self.last_record.key.into_owned_record_key(),
            next_iterator_key: next_record.key.into_owned_record_key(),
        })
    }

    fn inner_next(&mut self) -> Result<Option<K::Item>, RawDbError> {
        if self.is_empty {
            return Ok(None);
        }

        if self.records_seen >= self.records_to_view_limit {
            return Ok(None);
        }

        if self.next_record.is_none() {
            let last_record = mem::replace(&mut self.last_record, K::empty_kv_record());
            self.is_empty = true;

            if !is_need_to_push_last_record::<K>(
                self.generation_id.as_ref(),
                OwnedPhantomId::as_opt_ref(&self.phantom_id),
                &last_record,
            ) {
                return Ok(None);
            }

            return Ok(K::make_item(last_record));
        }

        let iterator = self
            .iterator
            .as_mut()
            .expect("There is no iterator only in finished state")
            .by_ref();

        let generation_id = self.generation_id.as_ref();
        let phantom_id = OwnedPhantomId::as_opt_ref(&self.phantom_id);

        let gc_phantom_id = self
            .gc_phantom_id
            .as_ref()
            .map(|x| PhantomId::new_unchecked(x.as_ref()));

        enum IteratorLoopResult<T> {
            Finished(Option<T>),
            End,
        }

        let iterator_loop_result = (|| {
            for kv in iterator {
                let (key, value) = kv?;

                self.records_seen += 1;

                let record_key = OwnedParsedRecordKey::from_boxed_slice(key)
                    .map_err(|_| RawDbError::InvalidRecordKey)?;

                let need_delete = is_gc_phantom(gc_phantom_id, record_key.get_phantom_id());
                let is_visible = !need_delete;

                if need_delete {
                    self.records_to_delete
                        .push(record_key.to_owned_record_key());
                }

                let new_next_record = IterationKvRecord {
                    key: record_key,
                    value: K::value_from_box(value),
                    is_visible,
                };

                let action = handle_last_and_next::<K, D>(
                    generation_id,
                    phantom_id,
                    &self.last_record,
                    self.next_record.as_ref().unwrap().key.get_parsed(),
                );

                match action {
                    LastAndNextHandleResult::PushLast => {
                        let old_next_record =
                            mem::replace(&mut self.next_record, Some(new_next_record));
                        let last_record =
                            mem::replace(&mut self.last_record, old_next_record.unwrap());

                        let item = K::make_item(last_record);
                        match item {
                            Some(item) => {
                                return Ok(IteratorLoopResult::Finished(Some(item)));
                            }
                            None => {}
                        }
                    }
                    LastAndNextHandleResult::ReplaceLastWithNext => {
                        let old_next_record =
                            mem::replace(&mut self.next_record, Some(new_next_record));
                        self.last_record = old_next_record.unwrap();
                    }
                    LastAndNextHandleResult::UpdateNext => {
                        self.next_record = Some(new_next_record);
                    }
                }

                if self.records_seen >= self.records_to_view_limit {
                    return Ok(IteratorLoopResult::Finished(None));
                }
            }

            Ok::<_, RawDbError>(IteratorLoopResult::End)
        })()?;

        match iterator_loop_result {
            IteratorLoopResult::Finished(result) => {
                return Ok(result);
            }
            IteratorLoopResult::End => {}
        }

        let action = handle_last_and_next::<K, D>(
            generation_id,
            phantom_id,
            &self.last_record,
            self.next_record.as_ref().unwrap().key.get_parsed(),
        );

        match action {
            LastAndNextHandleResult::PushLast => {
                let old_next_record = self.next_record.take();
                let last_record = mem::replace(&mut self.last_record, old_next_record.unwrap());

                let item = K::make_item(last_record);
                match item {
                    Some(item) => Ok(Some(item)),
                    None => {
                        let last_record = mem::replace(&mut self.last_record, K::empty_kv_record());

                        Ok(K::make_item(last_record))
                    }
                }
            }
            action => {
                let last_record = match action {
                    LastAndNextHandleResult::UpdateNext => {
                        self.next_record.take();
                        mem::replace(&mut self.last_record, K::empty_kv_record())
                    }
                    LastAndNextHandleResult::ReplaceLastWithNext => {
                        self.next_record.take().unwrap()
                    }
                    _ => panic!("impossible"),
                };

                self.is_empty = true;

                if !is_need_to_push_last_record::<K>(generation_id, phantom_id, &last_record) {
                    return Ok(None);
                }

                Ok(K::make_item(last_record))
            }
        }
    }
}

impl<'a, K: QueryKind, D: QueryDirection> Iterator for QueryState<'a, K, D> {
    type Item = Result<K::Item, RawDbError>;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.inner_next();

        match result {
            Ok(result) => result.map(|x| Ok(x)),
            Err(err) => Some(Err(err)),
        }
    }
}

enum LastAndNextHandleResult {
    PushLast,
    ReplaceLastWithNext,
    UpdateNext,
}

fn handle_last_and_next<K: QueryKind, D: QueryDirection>(
    generation_id: GenerationId<'_>,
    phantom_id: Option<PhantomId<'_>>,
    last_record: &IterationKvRecord<K::Value>,
    next_parsed_record: ParsedRecordKey<'_>,
) -> LastAndNextHandleResult {
    let last_parsed_record = last_record.key.get_parsed();

    let is_key_differs = last_parsed_record.collection_key != next_parsed_record.collection_key;

    if is_key_differs {
        if is_need_to_push_last_record::<K>(generation_id, phantom_id, last_record) {
            return LastAndNextHandleResult::PushLast;
        }

        return LastAndNextHandleResult::ReplaceLastWithNext;
    }

    if next_parsed_record.phantom_id.is_some() && next_parsed_record.phantom_id != phantom_id {
        return LastAndNextHandleResult::UpdateNext;
    }

    if !D::is_suitable_generation_id(
        generation_id,
        last_parsed_record.generation_id,
        next_parsed_record.generation_id,
    ) {
        return LastAndNextHandleResult::UpdateNext;
    }

    LastAndNextHandleResult::ReplaceLastWithNext
}

fn is_need_to_push_last_record<K: QueryKind>(
    generation_id: GenerationId<'_>,
    phantom_id: Option<PhantomId<'_>>,
    record: &IterationKvRecord<K::Value>,
) -> bool {
    let ParsedRecordKey {
        collection_key: _,
        generation_id: record_generation_id,
        phantom_id: record_phantom_id,
    } = record.key.get_parsed();

    if !record.is_visible {
        return false;
    }

    if record_phantom_id.is_some() && record_phantom_id != phantom_id {
        return false;
    }

    if record_generation_id > generation_id {
        return false;
    }

    true
}

fn create_iterator<'a, D: QueryDirection>(
    db: &'a DB,
    start_key: Option<CollectionKey<'_>>,
    _generation_id: GenerationId<'_>,
    _phantom_id: Option<PhantomId<'_>>,
    next_iterator_key: Option<RecordKey<'_>>,
) -> Result<DBIteratorWithThreadMode<'a, DB>, RawDbError> {
    match next_iterator_key {
        Some(next_iterator_key) => {
            let iterator_mode =
                IteratorMode::From(next_iterator_key.get_byte_array(), D::get_direction());
            Ok(db.iterator(iterator_mode))
        }
        None => match start_key {
            Some(start_key) => {
                let record_key = OwnedRecordKey::new(
                    start_key,
                    D::get_initial_record_generation_id(),
                    D::get_initial_record_phantom_id(),
                )
                .map_err(|_| RawDbError::InvalidRecordKey)?;

                let iterator_mode =
                    IteratorMode::From(record_key.get_byte_array(), D::get_direction());
                Ok(db.iterator(iterator_mode))
            }
            None => {
                let iterator_mode = D::get_default_iterator_mode();
                Ok(db.iterator(iterator_mode))
            }
        },
    }
}

enum InitializationResult<K: QueryKind> {
    End,
    Full {
        last_record: IterationKvRecord<K::Value>,
        next_record: Option<IterationKvRecord<K::Value>>,
    },
}

fn initialize_last_and_next<K: QueryKind>(
    db: &DB,
    iterator: &mut DBIteratorWithThreadMode<'_, DB>,
    last_candidate_key: Option<OwnedRecordKey>,
    gc_phantom_id: Option<PhantomId<'_>>,
) -> Result<InitializationResult<K>, RawDbError> {
    let is_continuation = last_candidate_key.is_some();

    let last_record = match last_candidate_key {
        Some(last_candidate_key) => {
            let value = db.get(last_candidate_key.get_byte_array())?;
            let Some(value) = value else {
                return Ok(InitializationResult::End);
            };

            let key = OwnedParsedRecordKey::from_owned_record_key(last_candidate_key);
            let is_visible = check_visibility(gc_phantom_id, key.get_phantom_id());

            IterationKvRecord {
                key,
                value: K::value_from_vec(value),
                is_visible,
            }
        }
        None => {
            let kv = iterator.next();
            let Some(kv) = kv else {
                return Ok(InitializationResult::End);
            };

            let (key, value) = kv?;

            let key = OwnedParsedRecordKey::from_boxed_slice(key)
                .map_err(|_| RawDbError::InvalidRecordKey)?;

            let is_visible = check_visibility(gc_phantom_id, key.get_phantom_id());

            IterationKvRecord {
                key,
                value: K::value_from_box(value),
                is_visible,
            }
        }
    };

    let kv = iterator.next();
    let Some(kv) = kv else {
        if is_continuation {
            return Ok(InitializationResult::End);
        }

        return Ok(InitializationResult::Full {
            last_record,
            next_record: None,
        });
    };

    let (key, value) = kv?;

    let key =
        OwnedParsedRecordKey::from_boxed_slice(key).map_err(|_| RawDbError::InvalidRecordKey)?;

    let is_visible = check_visibility(gc_phantom_id, key.get_phantom_id());

    let next_record = IterationKvRecord {
        key,
        value: K::value_from_box(value),
        is_visible,
    };

    Ok(InitializationResult::Full {
        last_record,
        next_record: Some(next_record),
    })
}

fn check_visibility(
    gc_phantom_id: Option<PhantomId<'_>>,
    record_phantom_id: Option<PhantomId<'_>>,
) -> bool {
    !is_gc_phantom(gc_phantom_id, record_phantom_id)
}

fn is_gc_phantom(
    gc_phantom_id: Option<PhantomId<'_>>,
    record_phantom_id: Option<PhantomId<'_>>,
) -> bool {
    let Some(gc_phantom_id) = gc_phantom_id else {
        return false;
    };

    let Some(record_phantom_id) = record_phantom_id else {
        return false;
    };

    record_phantom_id.get_byte_array() <= gc_phantom_id.get_byte_array()
}
