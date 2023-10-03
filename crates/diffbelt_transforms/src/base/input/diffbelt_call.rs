use diffbelt_types::collection::get::GetCollectionResponseJsonData;

pub struct DiffbeltCallInput {
    pub body: DiffbeltResponseBody,
}

pub enum DiffbeltResponseBody {
    GetCollection(GetCollectionResponseJsonData),
}
