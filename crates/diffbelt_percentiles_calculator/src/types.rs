use core::ops::Deref;

use alloc::string::String;

pub trait PTypes: Sized {
    type PercentileKey: Deref<Target = [u8]>;
    type TargetKey: Deref<Target = [u8]>;
    type TargetRecord: PercentilesTargetRecord<Self>;
    type DataProviderError;
    type FetchKeysAround: FetchKeysAroundResponse<Self>;
    type SourceRecord: SourceDiffRecord<Self>;
    type SourceChunk: PercentilesSourceChunk<Self>;
    type DataProvider: PercentilesDataProvider<Self>;
}

pub struct PercentileFull<P: PTypes> {
    pub p: f32,
    pub key: Option<P::PercentileKey>,
}

pub trait PercentilesTargetRecord<P: PTypes> {
    fn percentiles(&self) -> &[PercentileFull<P>];
}

pub enum FetchKeysAroundDirection {
    Left,
    Right,
    Both,
}

pub trait FetchKeysAroundResponse<P: PTypes> {
    fn center(&self) -> P::PercentileKey;
    fn direction() -> FetchKeysAroundDirection;

    fn left(&self) -> &[P::PercentileKey];
    fn right(&self) -> &[P::PercentileKey];
}

pub trait PercentilesDataProvider<P: PTypes> {
    fn get_target_record(&mut self, key: P::TargetKey) -> Result<u64, P::DataProviderError>;
    fn await_target_record(
        &mut self,
        id: u64,
    ) -> Result<Option<P::TargetRecord>, P::DataProviderError>;

    fn put_target_record(
        &mut self,
        key: P::TargetKey,
        record: P::TargetRecord,
    ) -> Result<(), P::DataProviderError>;

    fn fetch_keys_around(
        &mut self,
        key: P::PercentileKey,
        direction: FetchKeysAroundDirection,
    ) -> Result<u64, P::DataProviderError>;
    fn await_fetch_keys_around(
        &mut self,
        id: u64,
    ) -> Result<P::FetchKeysAround, P::DataProviderError>;

    fn insert_key(&mut self, key: P::PercentileKey) -> Result<u64, P::DataProviderError>;
    fn remove_key(&mut self, key: P::PercentileKey) -> Result<u64, P::DataProviderError>;

    fn is_async_completed(&self, id: u64) -> Result<bool, P::DataProviderError>;
    fn await_void_async(&mut self, id: u64) -> Result<(), P::DataProviderError>;
}

pub enum DiffKey<P: PTypes> {
    Added(P::PercentileKey),
    Removed(P::PercentileKey),
}

pub trait SourceDiffRecord<P: PTypes> {
    fn key(&self) -> &DiffKey<P>;
}

pub trait PercentilesSourceChunk<P: PTypes> {
    fn diffs<'a>(&'a self) -> impl Iterator<Item = &'a P::SourceRecord>
    where
        <P as PTypes>::SourceRecord: 'a;
}

pub struct PercentilesCalculatorOptions<P: PTypes> {
    pub chunk: P::SourceChunk,
    pub data_provider: P::DataProvider,
}

pub enum PercentilesError<P: PTypes> {
    DataProvider(P::DataProviderError),
    Message(String),
}

pub trait PercentilesCalculator<P: PTypes> {
    fn new(options: PercentilesCalculatorOptions<P>) -> Self;

    fn calculate(&mut self) -> Result<(), PercentilesError<P>>;
}
