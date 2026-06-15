use std::sync::Arc;

use diffbelt_percentiles_calculator::types::{
    DiffKey, PTypes, PercentileFull, PercentilesSourceChunk, PercentilesTargetRecord,
    SourceDiffRecord,
};

use crate::tests::data_provider::{MockDataProvider, MockDataProviderError, MockFetchAroundResponse};

mod basic;
pub mod data_provider;

#[derive(Clone)]
pub struct MockPTypes;

impl PTypes for MockPTypes {
    type PercentileKey = Arc<[u8]>;
    type TargetKey = Arc<[u8]>;

    type TargetRecord = Arc<MockTargetRecord>;

    type DataProviderError = MockDataProviderError;

    type FetchKeysAround = MockFetchAroundResponse;

    type SourceRecord = MockSourceRecord;

    type SourceChunk = MockSourceChunk;

    type DataProvider = MockDataProvider<MockPTypes>;
}

pub struct MockTargetRecord {
    pub percentiles: Vec<PercentileFull<MockPTypes>>,
}

impl PercentilesTargetRecord<MockPTypes> for Arc<MockTargetRecord> {
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
    fn diffs<'a>(&'a self) -> impl Iterator<Item = &'a <MockPTypes as PTypes>::SourceRecord>
    where
        <MockPTypes as PTypes>::SourceRecord: 'a,
    {
        self.records.iter()
    }
}
