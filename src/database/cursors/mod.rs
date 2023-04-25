use crate::database::config::DatabaseConfig;
use crate::database::cursors::thread::run;
use crate::messages::cursors::DatabaseCollectionCursorsTask;
use crate::util::async_task_thread::AsyncTaskThread;
use std::sync::Arc;

pub mod collection;
pub mod query;
pub mod storage;
mod thread;

pub async fn start_cursors_task_thread(
    config: Arc<DatabaseConfig>,
) -> AsyncTaskThread<DatabaseCollectionCursorsTask> {
    AsyncTaskThread::new(
        run,
        config,
        #[cfg(feature = "debug_prints")]
        "cursors",
    )
    .await
}
