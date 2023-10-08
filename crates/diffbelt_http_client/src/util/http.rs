use diffbelt_transforms::base::action::diffbelt_call::Method;
use hyper::Method as HyperMethod;

pub trait TransformMethodTrait {
    fn into_hyper_method(self) -> HyperMethod;
}

impl TransformMethodTrait for Method {
    fn into_hyper_method(self) -> HyperMethod {
        match self {
            Method::Get => HyperMethod::GET,
            Method::Post => HyperMethod::POST,
        }
    }
}
