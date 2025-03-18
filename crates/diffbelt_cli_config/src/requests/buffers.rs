use crate::requests::DiffbeltRequests;

impl DiffbeltRequests {
    pub fn take_request_buffer(&self) -> Vec<u8> {
        let mut buffers = self.request_buffers.lock().expect("lock");
        buffers.take()
    }

    pub fn return_request_buffer(&self, buffer: Vec<u8>) {
        let mut buffers = self.request_buffers.lock().expect("lock");
        buffers.push(buffer);
    }
}
