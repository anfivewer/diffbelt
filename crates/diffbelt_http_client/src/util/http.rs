use hyper::Method as HyperMethod;

use diffbelt_transforms::base::action::diffbelt_call::Method;

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
