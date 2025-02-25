pub mod buffers;
mod mocks;

use diffbelt_protos::protos::api::methods::{Request, Response};
use diffbelt_protos::protos::impls::{RequestProto, ResponseProto};
use diffbelt_protos::OwnedSerialized;
use diffbelt_util::Wrap;
use diffbelt_util_no_std::buffers_pool::BuffersPool;
use tokio::sync::{mpsc, oneshot};

type RequestData = OwnedSerialized<RequestProto>;
type ResponseData = OwnedSerialized<ResponseProto>;

pub struct DiffbeltRequests {
    sender: mpsc::Sender<(RequestData, oneshot::Sender<ResponseData>)>,
    request_buffers: std::sync::Mutex<BuffersPool<Vec<u8>>>,
}

impl DiffbeltRequests {
    pub fn new() -> (
        Self,
        mpsc::Receiver<(RequestData, oneshot::Sender<ResponseData>)>,
    ) {
        let (sender, receiver) = mpsc::channel(16);

        (
            Self {
                sender,
                request_buffers: Wrap::wrap(BuffersPool::with_capacity(4)),
            },
            receiver,
        )
    }

    pub async fn request(
        &self,
        data: OwnedSerialized<RequestProto>,
    ) -> Result<oneshot::Receiver<ResponseData>, ()> {
        let (sender, receiver) = oneshot::channel();

        let () = self.sender.send((data, sender)).await.map_err(|_| ())?;

        Ok(receiver)
    }
}
