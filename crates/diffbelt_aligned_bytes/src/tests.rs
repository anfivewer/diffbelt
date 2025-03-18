use crate::OwnedAlignedBytes;
use alloc::vec;

#[test]
fn test_owned_aligned() {
    let v = vec![1, 2, 3, 4];

    let aligned = OwnedAlignedBytes::<8>::new(v, 0, 4).expect("cannot create aligned bytes");

    assert_eq!(aligned.as_slice(), &[1, 2, 3, 4])
}

#[test]
fn test_how_align_works() {
    unsafe {
        let bytes = &[0u8; 24];
        let ptr = bytes.as_ptr();
        let ptr = if (ptr as usize) % 8 == 0 {
            ptr
        } else {
            ptr.add(8 - (ptr as usize) % 8)
        };

        let align_offset = ptr.align_offset(8);
        assert_eq!(align_offset, 0);
        
        let ptr = if (ptr as usize) % 8 == 0 {
            ptr.add(1)
        } else {
            ptr.add(8 - (ptr as usize) % 8 + 1)
        };

        assert_eq!((ptr as usize) % 8, 1);
        let align_offset = ptr.align_offset(8);
        assert_eq!(align_offset, 7);
    }
}

#[test]
fn test_write_bytes() {
    let mut aligned = OwnedAlignedBytes::<8>::with_capacity(21);
    let head_ptr = aligned.buffer.as_ptr();
    unsafe {
        let mut write = aligned.write_slice(5).expect("Cannot create write");
        assert_eq!(write.len, 5);
        assert_eq!((write.as_mut() as usize) % 8, 0);

        let head_offset = (head_ptr as usize) % 8;
        assert_eq!(
            if head_offset == 0 {
                head_ptr as usize
            } else {
                (head_ptr as usize) + (8 - head_offset)
            },
            write.as_mut() as usize
        );
    }
}
