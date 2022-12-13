use crate::collection::newgen::commit_next_generation::{
    commit_next_generation_sync, CommitNextGenerationSyncOptions,
};
use crate::collection::Collection;
use crate::common::NeverEq;
use crate::util::tokio::spawn_async_thread;

use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::watch;

pub mod commit_next_generation;

pub struct NewGenerationCommiter {
    stop_sender: Option<oneshot::Sender<()>>,
}

pub struct NewGenerationCommiterOptions {
    pub collection_receiver: oneshot::Receiver<Arc<Collection>>,
    pub on_put_receiver: watch::Receiver<NeverEq>,
}

impl NewGenerationCommiter {
    pub fn new(options: NewGenerationCommiterOptions) -> Self {
        let collection_receiver = options.collection_receiver;
        let on_put_receiver = options.on_put_receiver;
        let (stop_sender, stop_receiver) = oneshot::channel();

        let async_task = move || async {
            let mut stop_receiver = stop_receiver;
            let mut on_put_receiver = on_put_receiver;

            let collection = collection_receiver.await;
            let collection = match collection {
                Ok(collection) => collection,
                Err(_) => {
                    return;
                }
            };

            // For the first time we need to check anyway
            let need_create_next_generation = true;

            {
                on_put_receiver.borrow_and_update();
            }

            loop {
                if is_stopped(&mut stop_receiver) {
                    return;
                }

                if need_create_next_generation {
                    commit_next_generation_sync(CommitNextGenerationSyncOptions {
                        expected_generation_id: None,
                        raw_db: collection.raw_db.clone(),
                        meta_raw_db: collection.meta_raw_db.clone(),
                        generation_id: collection.generation_id.clone(),
                        next_generation_id: collection.next_generation_id.clone(),
                    })
                    .await
                    .unwrap_or(());
                }

                let result = on_put_receiver.changed().await;
                match result {
                    Ok(_) => {}
                    Err(_) => {
                        return;
                    }
                }
            }
        };

        // TODO: join on stop
        spawn_async_thread(async_task());

        NewGenerationCommiter {
            stop_sender: Some(stop_sender),
        }
    }
}

impl Drop for NewGenerationCommiter {
    fn drop(&mut self) {
        let stop_sender = self.stop_sender.take();

        match stop_sender {
            Some(stop_sender) => {
                stop_sender.send(()).unwrap_or(());
            }
            None => {}
        }
    }
}

fn is_stopped(stop_receiver: &mut oneshot::Receiver<()>) -> bool {
    let result = stop_receiver.try_recv();

    match result {
        Ok(_) => true,
        Err(err) => match err {
            oneshot::error::TryRecvError::Empty => false,
            oneshot::error::TryRecvError::Closed => true,
        },
    }
}
