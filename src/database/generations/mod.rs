mod thread;

use crate::database::generations::thread::run;
use crate::messages::generations::DatabaseCollectionGenerationsTask;
use crate::util::async_task_thread::AsyncTaskThread;

pub async fn start_generations_task_thread() -> AsyncTaskThread<DatabaseCollectionGenerationsTask> {
    AsyncTaskThread::new(
        run,
        (),
        #[cfg(feature = "debug_prints")]
        "generations",
    )
    .await
}
