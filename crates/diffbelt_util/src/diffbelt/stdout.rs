use crate::Wrap;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader, Lines};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use tracing::trace;

pub struct DiffbeltStdoutOptions<Read: AsyncRead> {
    pub read: Read,
}

#[derive(Error, Debug)]
pub enum DiffbeltStdoutError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Recv(#[from] watch::error::RecvError),
    #[error(transparent)]
    SendIdle(#[from] watch::error::SendError<bool>),
}

pub struct DiffbeltStdout {
    // lines: Lines<BufReader<Read>>,
    join_handle: JoinHandle<()>,
    error: Arc<std::sync::Mutex<Option<DiffbeltStdoutError>>>,
    pub idle_receiver: watch::Receiver<bool>,
}

impl DiffbeltStdout {
    pub fn new<Read: AsyncRead + Unpin + Send + 'static>(
        options: DiffbeltStdoutOptions<Read>,
    ) -> Self {
        let buf_reader = BufReader::new(options.read);
        let mut lines = buf_reader.lines();

        let error: Arc<std::sync::Mutex<Option<DiffbeltStdoutError>>> = Wrap::wrap(None);
        let (idle_sender, idle_receiver) = watch::channel(false);

        let task_error = error.clone();
        let join_handle = tokio::spawn(async move {
            let result = (|| async move {
                while let Some(line) = lines.next_line().await? {
                    trace!(target: "diffbelt", "{line}");

                    if line.as_str().contains("  INFO diffbeltIdleStatus: IDLE") {
                        let () = idle_sender.send(true)?;
                    } else if line.as_str().contains("  INFO diffbeltIdleStatus: BUSY") {
                        let () = idle_sender.send(false)?;
                    }
                }

                Ok::<_, DiffbeltStdoutError>(())
            })()
            .await;

            if let Err(err) = result {
                let Ok(mut task_error) = task_error.lock() else {
                    return;
                };
                *task_error = Some(err);
            }
        });

        Self {
            join_handle,
            error,
            idle_receiver,
        }
    }

    pub async fn wait_for_idle(&mut self) -> Result<(), DiffbeltStdoutError> {
        loop {
            let () = self.idle_receiver.changed().await?;

            let is_idle = *self.idle_receiver.borrow_and_update();
            if !is_idle {
                continue;
            }

            sleep(Duration::from_millis(100)).await;

            let is_idle = *self.idle_receiver.borrow_and_update();
            if is_idle {
                return Ok(());
            }
        }
    }
}

impl Drop for DiffbeltStdout {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}
