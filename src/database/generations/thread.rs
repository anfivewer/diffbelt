use crate::messages::generations::DatabaseCollectionGenerationsTask;
use crate::util::async_task_thread::TaskPoller;

pub async fn run(_: (), mut poller: TaskPoller<DatabaseCollectionGenerationsTask>) {
    let task = poller.poll().await;
    let Some(task) = task else {
        return;
    };

    let DatabaseCollectionGenerationsTask::Init(_database) = task else {
        panic!("database/generations/thread first task is not init");
    };

    //
}
