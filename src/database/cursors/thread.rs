use crate::messages::cursors::DatabaseCollectionCursorsTask;
use crate::util::async_task_thread::TaskPoller;

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionCursorsTask>) {
    while let Some(_task) = poller.poll().await {
        //
    }
}
