use text_diff::diff;
use crate::config_tests::error::{AssertError, TestError};

pub(crate) fn compare_strings(expected: &str, actual: &str) -> Option<AssertError> {
    let (distance, diffs) = diff(expected, actual, "\n");

    let (distance, diffs) = if distance != 0 && diffs.len() == 1 {
        diff(expected, actual, "")
    } else {
        (distance, diffs)
    };

    if distance == 0 {
        return None;
    }

    Some(AssertError::HasDiff { diffs })
}