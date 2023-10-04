use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::get::GetCollectionResponseJsonData;

pub struct DiffbeltCallInput<T> {
    pub body: T,
}

pub enum DiffbeltResponseBody {
    Ok(()),
    GetCollection(GetCollectionResponseJsonData),
    Diff(DiffCollectionResponseJsonData),
}
