use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::get::GetCollectionResponseJsonData;
use diffbelt_types::collection::put_many::PutManyResponseJsonData;

pub struct DiffbeltCallInput<T> {
    pub body: T,
}

#[derive(Debug)]
pub enum DiffbeltResponseBody {
    Ok(()),
    GetCollection(GetCollectionResponseJsonData),
    Diff(DiffCollectionResponseJsonData),
    PutMany(PutManyResponseJsonData),
}
