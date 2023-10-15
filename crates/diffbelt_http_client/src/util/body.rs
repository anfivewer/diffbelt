use diffbelt_transforms::base::action::diffbelt_call::DiffbeltRequestBody;
use hyper::Body;

pub enum ExpectedResponseType {
    Ok,
    Diff,
    PutMany,
}

pub trait TransformBodyTrait {
    fn into_hyper_body(self) -> Result<(Body, ExpectedResponseType), serde_json::Error>;
}

impl TransformBodyTrait for DiffbeltRequestBody {
    fn into_hyper_body(self) -> Result<(Body, ExpectedResponseType), serde_json::Error> {
        let (body, expected_response_type) = match self {
            DiffbeltRequestBody::ReadDiffCursorNone => {
                return Ok((Body::empty(), ExpectedResponseType::Diff));
            }
            DiffbeltRequestBody::DiffCollectionStart(x) => {
                (serde_json::to_string(&x), ExpectedResponseType::Diff)
            }
            DiffbeltRequestBody::StartGeneration(x) => {
                (serde_json::to_string(&x), ExpectedResponseType::Ok)
            }
            DiffbeltRequestBody::CommitGeneration(x) => {
                (serde_json::to_string(&x), ExpectedResponseType::Ok)
            }
            DiffbeltRequestBody::PutMany(x) => {
                (serde_json::to_string(&x), ExpectedResponseType::PutMany)
            }
        };

        let body = body?;

        Ok((Body::from(body), expected_response_type))
    }
}
