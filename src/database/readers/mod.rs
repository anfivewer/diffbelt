use crate::database::readers::thread::run;
use crate::messages::readers::DatabaseCollecitonReadersTask;
use crate::util::async_task_thread::AsyncTaskThread;

mod thread;

pub async fn start_readers_task_thread() -> AsyncTaskThread<DatabaseCollecitonReadersTask> {
    AsyncTaskThread::new(run, ()).await
}
