use crate::common::GenerationId;

enum CollectionReaderCollectionId {
    ThisCollection,
    Collection(String),
}

struct CollectionReader {
    reader_id: String,
    generation_id: GenerationId,
    collection_id: CollectionReaderCollectionId,
}
