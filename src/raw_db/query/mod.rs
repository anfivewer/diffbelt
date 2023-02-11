use crate::collection::util::record_key::{OwnedParsedRecordKey, OwnedRecordKey, ParsedRecordKey};
use crate::common::{
    CollectionKey, CollectionValue, GenerationId, IsByteArray, OwnedGenerationId, OwnedPhantomId,
    PhantomId,
};
use crate::raw_db::RawDbError;
use rocksdb::{DBIteratorWithThreadMode, Direction, IteratorMode, DB};

use std::mem;

pub struct QueryKeyValue;
pub struct QueryKeysOnly;

pub struct QueryDirectionBackward;
pub struct QueryDirectionForward;

pub struct ContinuationState {
    last_candidate_key: OwnedRecordKey,
    next_iterator_key: OwnedRecordKey,
}

pub struct QueryOptions<'a, K: QueryKind, D: QueryDirection> {
    pub kind: K,
    pub direction: D,
    pub start_key: Option<CollectionKey<'a>>,
    pub generation_id: GenerationId<'a>,
    pub phantom_id: Option<PhantomId<'a>>,
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
}

pub struct QueryState<'a, K: QueryKind, D: QueryDirection> {
    is_empty: bool,
    kind: K,
    direction: D,
    generation_id: OwnedGenerationId,
    phantom_id: Option<OwnedPhantomId>,
    records_seen: usize,
    records_to_view_limit: usize,
    iterator: Option<DBIteratorWithThreadMode<'a, DB>>,
    last_record: IterationKvRecord<K::Value>,
    next_record: Option<IterationKvRecord<K::Value>>,
}

impl<'a, K: QueryKind, D: QueryDirection> QueryState<'a, K, D> {
    pub fn new(db: &'a DB, options: QueryOptions<'_, K, D>) -> Result<Self, RawDbError> {
        let QueryOptions {
            kind,
            direction,
            start_key,
            generation_id,
            phantom_id,
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

        let mut iterator =
            create_iterator::<D>(db, start_key, generation_id, phantom_id, next_iterator_key)?;

        let initialization_result =
            initialize_last_and_next::<K>(db, &mut iterator, last_candidate_key)?;

        match initialization_result {
            InitializationResult::End => Ok(Self {
                is_empty: true,
                kind,
                direction,
                generation_id: generation_id.to_owned(),
                phantom_id: None,
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
                records_seen: 0,
                records_to_view_limit,
                iterator: Some(iterator),
                last_record,
                next_record,
            }),
        }
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

            if !is_key_satisties(
                self.generation_id.as_ref(),
                OwnedPhantomId::as_opt_ref(&self.phantom_id),
                last_record.key.get_parsed(),
            ) {
                return Ok(None);
            }

            return Ok(K::make_item(last_record));
        }

        let iterator = self.iterator.as_mut().unwrap().by_ref();

        for kv in iterator {
            let (key, value) = kv?;

            self.records_seen += 1;

            let new_next_record = IterationKvRecord {
                key: OwnedParsedRecordKey::from_boxed_slice(key)
                    .map_err(|_| RawDbError::InvalidRecordKey)?,
                value: K::value_from_box(value),
            };

            let ParsedRecordKey {
                collection_key: last_collection_key,
                generation_id: last_generation_id,
                phantom_id: last_phantom_id,
            } = self.last_record.key.get_parsed();

            let ParsedRecordKey {
                collection_key: next_collection_key,
                generation_id: next_generation_id,
                phantom_id: next_phantom_id,
            } = self.next_record.as_ref().unwrap().key.get_parsed();

            let is_key_differs = last_collection_key != next_collection_key;

            if is_key_differs {
                if is_key_satisties(
                    self.generation_id.as_ref(),
                    OwnedPhantomId::as_opt_ref(&self.phantom_id),
                    self.last_record.key.get_parsed(),
                ) {
                    let old_next_record =
                        mem::replace(&mut self.next_record, Some(new_next_record));
                    let last_record = mem::replace(&mut self.last_record, old_next_record.unwrap());

                    return Ok(K::make_item(last_record));
                }

                let old_next_record = mem::replace(&mut self.next_record, Some(new_next_record));
                self.last_record = old_next_record.unwrap();

                if self.records_seen >= self.records_to_view_limit {
                    return Ok(None);
                }
                continue;
            }

            let should_skip = last_phantom_id != next_phantom_id
                || !D::is_suitable_generation_id(
                    self.generation_id.as_ref(),
                    last_generation_id,
                    next_generation_id,
                );

            if should_skip {
                self.next_record = Some(new_next_record);

                if self.records_seen >= self.records_to_view_limit {
                    return Ok(None);
                }
                continue;
            }

            let old_next_record = mem::replace(&mut self.next_record, Some(new_next_record));
            self.last_record = old_next_record.unwrap();

            if self.records_seen >= self.records_to_view_limit {
                return Ok(None);
            }
        }

        let last_record = mem::replace(&mut self.last_record, K::empty_kv_record());
        self.is_empty = true;
        self.next_record.take();
        self.iterator.take();

        if !is_key_satisties(
            self.generation_id.as_ref(),
            OwnedPhantomId::as_opt_ref(&self.phantom_id),
            last_record.key.get_parsed(),
        ) {
            return Ok(None);
        }

        Ok(K::make_item(last_record))
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

fn is_key_satisties(
    generation_id: GenerationId<'_>,
    phantom_id: Option<PhantomId<'_>>,
    key: ParsedRecordKey<'_>,
) -> bool {
    let ParsedRecordKey {
        collection_key: _,
        generation_id: record_generation_id,
        phantom_id: record_phantom_id,
    } = key;

    if record_phantom_id != phantom_id {
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
    generation_id: GenerationId<'_>,
    phantom_id: Option<PhantomId<'_>>,
    next_iterator_key: Option<OwnedRecordKey>,
) -> Result<DBIteratorWithThreadMode<'a, DB>, RawDbError> {
    match next_iterator_key {
        Some(next_iterator_key) => {
            let iterator_mode =
                IteratorMode::From(next_iterator_key.get_byte_array(), D::get_direction());
            Ok(db.iterator(iterator_mode))
        }
        None => match start_key {
            Some(start_key) => {
                let record_key =
                    OwnedRecordKey::new(start_key, generation_id, PhantomId::or_empty(&phantom_id))
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
) -> Result<InitializationResult<K>, RawDbError> {
    let is_continuation = last_candidate_key.is_some();

    let last_record = match last_candidate_key {
        Some(last_candidate_key) => {
            let value = db.get(last_candidate_key.get_byte_array())?;
            let Some(value) = value else {
                return Err(RawDbError::CursorDidNotFoundRecord);
            };

            IterationKvRecord {
                key: OwnedParsedRecordKey::from_owned_record_key(last_candidate_key),
                value: K::value_from_vec(value),
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

            IterationKvRecord {
                key,
                value: K::value_from_box(value),
            }
        }
    };

    let kv = iterator.next();
    let Some(kv) = kv else {
        if is_continuation {
            return Err(RawDbError::CursorDidNotFoundRecord);
        }

        return Ok(InitializationResult::Full {
            last_record,
            next_record: None,
        });
    };

    let (key, value) = kv?;

    let key =
        OwnedParsedRecordKey::from_boxed_slice(key).map_err(|_| RawDbError::InvalidRecordKey)?;

    let next_record = IterationKvRecord {
        key,
        value: K::value_from_box(value),
    };

    Ok(InitializationResult::Full {
        last_record,
        next_record: Some(next_record),
    })
}
