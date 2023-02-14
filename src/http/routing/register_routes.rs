use crate::context::Context;
use crate::http::routing::routes::collection::by_id::register_collection_by_id_route;
use crate::http::routing::routes::collection::create::register_create_collection_route;
use crate::http::routing::routes::collection::generation_id_stream::register_collection_generation_id_stream_route;
use crate::http::routing::routes::collection::list::register_list_collections_route;
use crate::http::routing::routes::diff::abort::register_abort_diff_route;
use crate::http::routing::routes::diff::next::register_next_diff_route;
use crate::http::routing::routes::diff::start::register_start_diff_route;
use crate::http::routing::routes::generation::abort::register_abort_generation_route;
use crate::http::routing::routes::generation::commit::register_commit_generation_route;
use crate::http::routing::routes::generation::start::register_start_generation_route;
use crate::http::routing::routes::get::register_get_route;
use crate::http::routing::routes::get_keys_around::register_get_keys_around_route;
use crate::http::routing::routes::get_many::register_get_many_route;
use crate::http::routing::routes::put::register_put_route;
use crate::http::routing::routes::put_many::register_put_many_route;
use crate::http::routing::routes::query::abort::register_abort_query_route;
use crate::http::routing::routes::query::next::register_next_query_route;
use crate::http::routing::routes::query::start::register_start_query_route;
use crate::http::routing::routes::reader::create::register_create_reader_route;
use crate::http::routing::routes::reader::delete::register_delete_reader_route;
use crate::http::routing::routes::reader::list::register_list_readers_route;
use crate::http::routing::routes::reader::update::register_update_reader_route;
use crate::http::routing::routes::root::register_root_route;

pub fn register_routes(context: &mut Context) {
    register_root_route(context);
    register_get_route(context);
    register_get_many_route(context);
    register_get_keys_around_route(context);
    register_put_route(context);
    register_put_many_route(context);
    register_collection_by_id_route(context);
    register_collection_generation_id_stream_route(context);
    register_list_collections_route(context);
    register_create_collection_route(context);
    register_create_reader_route(context);
    register_list_readers_route(context);
    register_update_reader_route(context);
    register_delete_reader_route(context);
    register_start_generation_route(context);
    register_abort_generation_route(context);
    register_commit_generation_route(context);
    register_start_query_route(context);
    register_next_query_route(context);
    register_abort_query_route(context);
    register_start_diff_route(context);
    register_next_diff_route(context);
    register_abort_diff_route(context);
}
