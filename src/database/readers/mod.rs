use crate::database::readers::thread::run;
use crate::messages::readers::DatabaseCollectionReadersTask;
use crate::util::async_task_thread::AsyncTaskThread;

mod thread;

pub async fn start_readers_task_thread() -> AsyncTaskThread<DatabaseCollectionReadersTask> {
    AsyncTaskThread::new(run, ()).await
}
