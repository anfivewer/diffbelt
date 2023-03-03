use crate::database::cursors::thread::run;
use crate::messages::cursors::DatabaseCollectionCursorsTask;
use crate::util::async_task_thread::AsyncTaskThread;

mod thread;

pub async fn start_cursors_task_thread() -> AsyncTaskThread<DatabaseCollectionCursorsTask> {
    AsyncTaskThread::new(run, ()).await
}
