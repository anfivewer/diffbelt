mod collection;
mod thread;

use crate::database::garbage_collector::thread::run;
use crate::messages::garbage_collector::DatabaseGarbageCollectorTask;
use crate::util::async_task_thread::AsyncTaskThread;

pub async fn start_garbage_collector_task_thread() -> AsyncTaskThread<DatabaseGarbageCollectorTask>
{
    AsyncTaskThread::new(
        run,
        (),
        #[cfg(feature = "debug_prints")]
        "garbage_collector",
    )
    .await
}
