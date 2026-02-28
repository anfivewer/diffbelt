use alloc::{rc::Rc, vec::Vec};

use crate::{
    tests::data_provider::{MockDataProviderError, MockFetchAroundResponse},
    types::{
        DiffKey, PTypes, PercentileFull, PercentilesSourceChunk, PercentilesTargetRecord, SourceDiffRecord
    },
};

pub mod data_provider;

pub struct MockPTypes;

impl PTypes for MockPTypes {
    type PercentileKey = Rc<[u8]>;
    type TargetKey = Rc<[u8]>;

    type TargetRecord = MockTargetRecord;

    type DataProviderError = MockDataProviderError;

    type FetchKeysAround = MockFetchAroundResponse;

    type SourceRecord = MockSourceRecord;

    type SourceChunk = MockSourceChunk;
}

pub struct MockTargetRecord {
    pub percentiles: Vec<PercentileFull<MockPTypes>>,
}

impl PercentilesTargetRecord<MockPTypes> for MockTargetRecord {
    fn percentiles(&self) -> &[PercentileFull<MockPTypes>] {
        &self.percentiles
    }
}

pub struct MockSourceRecord {
    pub diff_key: DiffKey<MockPTypes>,
}

impl SourceDiffRecord<MockPTypes> for MockSourceRecord {
    fn key(&self) -> &DiffKey<MockPTypes> {
        &self.diff_key
    }
}

pub struct MockSourceChunk {
    pub records: Vec<MockSourceRecord>,
}

impl PercentilesSourceChunk<MockPTypes> for MockSourceChunk {
    fn diffs<'a>(&'a self) -> impl Iterator<Item = &'a <MockPTypes as PTypes>::SourceRecord>  where <MockPTypes as PTypes>::SourceRecord: 'a {
        self.records.iter()
    }
}
