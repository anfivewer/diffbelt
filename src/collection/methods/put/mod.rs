pub use crate::collection::methods::put::put_many::{
    CollectionPutManyOk, CollectionPutManyOptions, CollectionPutManyResult,
};
pub use crate::collection::methods::put::put_single::{
    CollectionPutOk, CollectionPutOptions, CollectionPutResult,
};

mod inner;
pub mod put_many;
pub mod put_single;
