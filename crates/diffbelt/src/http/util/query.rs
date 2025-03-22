use crate::collection::methods::query::QueryOk;
use crate::common::IsByteArray;
use diffbelt_protos::protos::api::common::{KeyValue, KeyValueArgs};
use diffbelt_protos::protos::api::query::QueryResponseArgs;
use diffbelt_protos::protos::impls::ResponseProto;
use diffbelt_protos::Serializer;

pub fn serialize_query_ok<'a>(
    serializer: &mut Serializer<'a, ResponseProto>,
    result: QueryOk,
) -> QueryResponseArgs<'a> {
    let QueryOk {
        generation_id,
        items,
        cursor_id,
    } = result;

    let generation_id = Some(serializer.create_vector(generation_id.get_byte_array()));
    let cursor_id = cursor_id.map(|id| serializer.create_string(&id));

    let items: Vec<_> = items
        .into_iter()
        .filter(|kv| !kv.value.is_empty())
        .map(|kv| {
            let key = Some(serializer.create_vector(kv.key.get_byte_array()));
            let value = Some(serializer.create_vector(kv.value.get_value()));

            KeyValue::create(serializer.buffer_builder(), &KeyValueArgs { key, value })
        })
        .collect();
    let items = Some(serializer.create_vector(&items));

    QueryResponseArgs {
        generation_id,
        cursor_id,
        items,
    }
}
