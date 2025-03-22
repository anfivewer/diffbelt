use alloc::vec::Vec;
use core::mem;
use diffbelt_protos::protos::api::common::KeyValue;
use diffbelt_protos::protos::api::query::{NextQueryRequestArgs, StartQueryRequestArgs};
use diffbelt_protos::protos::handlers::{NextQueryApiHandler, StartQueryApiHandler};
use diffbelt_protos::Serializer;
use diffbelt_wasm_binding::requests::Request;

pub fn query_intermediate<F: FnMut(KeyValue<'_>)>(
    buffer_holder: &mut Option<Vec<u8>>,
    buffer_holder2: &mut Option<Vec<u8>>,
    mut handler: F,
) {
    let mut serializer = Serializer::from_vec(buffer_holder.take().unwrap_or_default());
    let collection_name = Some(serializer.create_string("updateMs:1d:intermediate"));
    let mut request = Request::<StartQueryApiHandler>::call(
        serializer,
        StartQueryRequestArgs {
            collection_name,
            generation_id: None,
            phantom_id: None,
        },
    )
    .expect("request");
    let response = request.on_request_finished().expect("response");
    let response_data = response.response().expect("response");

    for item in response_data.items().unwrap_or_default() {
        handler(item);
    }

    let Some(cursor_id) = response_data.cursor_id() else {
        *buffer_holder = request.take_buffer().map(|x| x.into_underlying_vec());
        return;
    };

    let mut serializer = Serializer::from_vec(buffer_holder2.take().unwrap_or_default());
    let collection_name = Some(serializer.create_string("updateMs:1d:intermediate"));
    let cursor_id = Some(serializer.create_string(cursor_id));
    *buffer_holder = request.take_buffer().map(|x| x.into_underlying_vec());
    let mut request = Request::<NextQueryApiHandler>::call(
        serializer,
        NextQueryRequestArgs {
            collection_name,
            cursor_id,
        },
    )
    .expect("request");
    let mut response = request.on_request_finished().expect("response");

    let mut free_holder = buffer_holder;
    let mut empty_holder = buffer_holder2;

    loop {
        let response_data = response.response().expect("response");

        for item in response_data.items().unwrap_or_default() {
            handler(item);
        }

        let Some(cursor_id) = response_data.cursor_id() else {
            *empty_holder = request.take_buffer().map(|x| x.into_underlying_vec());
            return;
        };

        let mut serializer = Serializer::from_vec(free_holder.take().unwrap_or_default());
        let collection_name = Some(serializer.create_string("updateMs:1d:intermediate"));
        let cursor_id = Some(serializer.create_string(cursor_id));
        {
            *empty_holder = request.take_buffer().map(|x| x.into_underlying_vec());
            mem::swap(free_holder, empty_holder);
        }
        request = Request::<NextQueryApiHandler>::call(
            serializer,
            NextQueryRequestArgs {
                collection_name,
                cursor_id,
            },
        )
        .expect("request");
        response = request.on_request_finished().expect("response");
    }
}
