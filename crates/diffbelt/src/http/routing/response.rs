pub struct BaseResponse {
    pub status: u16,
    pub content_type: &'static str,
}

impl Default for BaseResponse {
    fn default() -> Self {
        Self {
            status: 200,
            content_type: "text/plain",
        }
    }
}

pub struct StaticStrResponse {
    pub base: BaseResponse,
    pub str: &'static str,
}

pub struct StringResponse {
    pub base: BaseResponse,
    pub str: String,
}

pub struct BytesVecResponse {
    pub base: BaseResponse,
    pub bytes: Vec<u8>,
}

pub enum Response {
    StaticStr(StaticStrResponse),
    String(StringResponse),
    BytesVec(BytesVecResponse),
}
