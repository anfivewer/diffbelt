use crate::OwnedAlignedBytes;
use alloc::vec;

#[test]
fn test_owned_aligned() {
    let v = vec![1, 2, 3, 4];

    let aligned = OwnedAlignedBytes::<8>::new(v, 0, 4).expect("cannot create aligned bytes");

    assert_eq!(aligned.as_slice(), &[1, 2, 3, 4])
}
