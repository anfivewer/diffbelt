use crate::base::error::TransformError;
use crate::base::input::diffbelt_call::{DiffbeltCallInput, DiffbeltResponseBody};
use crate::base::input::function_eval::{
    AggregateInitialAccumulatorEvalInput, AggregateMapEvalInput, AggregateMergeEvalInput,
    AggregateReduceEvalInput, AggregateTargetInfoEvalInput, FunctionEvalInput,
    FunctionEvalInputBody, MapFilterEvalInput,
};
use crate::base::input::InputType;
use diffbelt_types::collection::diff::DiffCollectionResponseJsonData;
use diffbelt_types::collection::get_record::GetResponseJsonData;
use diffbelt_types::collection::put_many::PutManyResponseJsonData;

macro_rules! input_type_into_diffbelt {
    ( $method_name:ident, $t:ty, $body_variant:ident ) => {
        pub fn $method_name(self) -> Result<DiffbeltCallInput<$t>, TransformError> {
            let InputType::DiffbeltCall(call) = self else {
                return Err(TransformError::Unspecified(
                    "Unexpected input, expected DiffbeltCall".to_string(),
                ));
            };

            let DiffbeltCallInput { body } = call;

            let DiffbeltResponseBody::$body_variant(body) = body else {
                return Err(TransformError::Unspecified(format!(
                    "Unexpected input, expected {}",
                    stringify!($body_variant)
                )));
            };

            Ok(DiffbeltCallInput { body })
        }
    };
}

macro_rules! input_type_into_eval {
    ( $method_name:ident, $t:ty, $body_variant:ident ) => {
        pub fn $method_name(self) -> Result<FunctionEvalInput<$t>, TransformError> {
            let InputType::FunctionEval(input) = self else {
                return Err(TransformError::Unspecified(
                    "Unexpected input, expected FunctionEval".to_string(),
                ));
            };

            let FunctionEvalInput { body } = input;

            let FunctionEvalInputBody::$body_variant(body) = body else {
                return Err(TransformError::Unspecified(format!(
                    "Unexpected input, expected {}",
                    stringify!($body_variant)
                )));
            };

            Ok(FunctionEvalInput { body })
        }
    };
}

impl InputType {
    input_type_into_diffbelt!(into_diffbelt_ok, (), Ok);
    input_type_into_diffbelt!(into_diffbelt_diff, DiffCollectionResponseJsonData, Diff);
    input_type_into_diffbelt!(into_diffbelt_put_many, PutManyResponseJsonData, PutMany);
    input_type_into_diffbelt!(into_diffbelt_get_record, GetResponseJsonData, GetRecord);

    input_type_into_eval!(into_eval_map_filter, MapFilterEvalInput, MapFilter);
    input_type_into_eval!(into_eval_aggregate_map, AggregateMapEvalInput, AggregateMap);
    input_type_into_eval!(
        into_eval_aggregate_target_info,
        AggregateTargetInfoEvalInput,
        AggregateTargetInfo
    );
    input_type_into_eval!(
        into_eval_aggregate_initial_accumulator,
        AggregateInitialAccumulatorEvalInput,
        AggregateInitialAccumulator
    );
    input_type_into_eval!(
        into_eval_aggregate_reduce,
        AggregateReduceEvalInput,
        AggregateReduce
    );
    input_type_into_eval!(
        into_eval_aggregate_merge,
        AggregateMergeEvalInput,
        AggregateMerge
    );
}
