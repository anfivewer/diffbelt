use crate::common::IsByteArray;

pub fn is_byte_array_equal<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: A,
    other: B,
) -> bool {
    let a: &[u8] = this.into();
    let b: &[u8] = other.into();
    a == b
}

pub fn is_byte_array_equal_opt<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: A,
    other: Option<B>,
) -> bool {
    match other {
        Some(other) => is_byte_array_equal(this, other),
        None => false,
    }
}

pub fn is_byte_array_equal_both_opt<'a, 'b, A: Into<&'a [u8]>, B: Into<&'b [u8]>>(
    this: Option<A>,
    other: Option<B>,
) -> bool {
    match this {
        Some(this) => is_byte_array_equal_opt(this, other),
        None => other.is_none(),
    }
}
