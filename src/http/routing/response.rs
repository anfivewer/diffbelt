pub struct BaseResponse {
    pub status: u16,
}

impl Default for BaseResponse {
    fn default() -> Self {
        Self { status: 200 }
    }
}

pub struct StringResponse {
    pub base: BaseResponse,
    pub str: String,
}

pub enum Response {
    String(StringResponse),
}
