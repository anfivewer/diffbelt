use crate::common::IsByteArray;

pub fn is_byte_array_equal<T: IsByteArray>(this: &T, other: &T) -> bool {
    this.get_byte_array() == other.get_byte_array()
}

pub fn is_byte_array_equal_opt<T: IsByteArray>(this: &T, other: Option<&T>) -> bool {
    match other {
        Some(other) => is_byte_array_equal(this, other),
        None => false,
    }
}

pub fn is_byte_array_equal_both_opt<T: IsByteArray>(this: Option<&T>, other: Option<&T>) -> bool {
    match this {
        Some(this) => is_byte_array_equal_opt(this, other),
        None => other.is_none(),
    }
}
