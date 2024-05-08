use crate::collection::methods::query::{QueryOk, QueryOptions, ReadQueryCursorOptions};
use crate::collection::Collection;
use crate::common::{GenerationId, KeyValue, OwnedGenerationId, OwnedPhantomId};
use futures::future::BoxFuture;

pub async fn assert_query(
    collection: &Collection,
    quering_generation_id: Option<OwnedGenerationId>,
    quering_phatom_id: Option<OwnedPhantomId>,
    expected_generation_id: GenerationId<'_>,
    expected_items: &[KeyValue],
) {
    assert_query_inner(
        collection,
        quering_generation_id,
        quering_phatom_id,
        expected_generation_id,
        expected_items,
        None,
    )
    .await
}

fn assert_query_inner<'a>(
    collection: &'a Collection,
    quering_generation_id: Option<OwnedGenerationId>,
    quering_phatom_id: Option<OwnedPhantomId>,
    expected_generation_id: GenerationId<'a>,
    expected_items: &'a [KeyValue],
    cursor_id: Option<Box<str>>,
) -> BoxFuture<'a, ()> {
    let fut = async move {
        let result = match &cursor_id {
            Some(cursor_id) => collection
                .read_query_cursor(ReadQueryCursorOptions {
                    cursor_id: cursor_id.clone(),
                })
                .await
                .unwrap(),
            None => collection
                .query(QueryOptions {
                    generation_id: quering_generation_id.clone(),
                    phantom_id: quering_phatom_id.clone(),
                })
                .await
                .unwrap(),
        };

        let QueryOk {
            generation_id: actual_generation_id,
            items: actual_items,
            cursor_id: next_cursor_id,
        } = result;

        assert_eq!(actual_generation_id.as_ref(), expected_generation_id);

        // WARN: when custom limits on packs will be implemented, it can fail
        assert!(actual_items.len() <= 200);

        let this_pack_expected_items = &expected_items[0..(actual_items.len())];
        let next_pack_expected_items = &expected_items[(actual_items.len())..];

        assert_eq!(&actual_items, this_pack_expected_items);

        if cursor_id.is_some() {
            assert!(next_cursor_id.is_some());
        } else {
            assert_eq!(
                next_cursor_id.is_none(),
                next_pack_expected_items.is_empty()
            );
        }

        if next_pack_expected_items.is_empty() {
            if cursor_id.is_some() {
                let QueryOk {
                    generation_id: actual_generation_id,
                    items,
                    cursor_id,
                } = collection
                    .read_query_cursor(ReadQueryCursorOptions {
                        cursor_id: next_cursor_id.unwrap(),
                    })
                    .await
                    .unwrap();

                assert_eq!(actual_generation_id.as_ref(), expected_generation_id);
                assert!(items.is_empty());
                assert!(cursor_id.is_none());
            }

            return ();
        }

        assert_query_inner(
            collection,
            quering_generation_id,
            quering_phatom_id,
            expected_generation_id,
            next_pack_expected_items,
            next_cursor_id,
        )
        .await
    };

    Box::pin(fut)
}
