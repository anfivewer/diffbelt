#![no_std]

#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub struct OwnedAlignedBytes<const ALIGN: usize> {
    buffer: Vec<u8>,
    head: usize,
    len: usize,
}

#[derive(Copy, Clone)]
pub struct AlignedBytes<'a, const ALIGN: usize>(&'a [u8]);

#[derive(Debug)]
pub struct AlignedBytesError {
    pub reason: String,
    pub buffer: Option<Vec<u8>>,
}

impl<'a, const ALIGN: usize> AlignedBytes<'a, ALIGN> {
    pub fn ensure_alignment_or_copy(
        bytes: &'a [u8],
        buffer: &'a mut Vec<u8>,
    ) -> Result<Self, AlignedBytesError> {
        if bytes.is_empty() {
            return Ok(Self(bytes));
        }

        // SAFETY: just checked
        let ptr = unsafe { bytes.get_unchecked(0) as *const u8 };
        let offset = ptr.align_offset(ALIGN);

        if offset == usize::MAX {
            return Err(AlignedBytesError {
                reason: format!("AlignedBytes: impossible to align({ALIGN}) ptr({ptr:p})"),
                buffer: None,
            });
        }

        if offset == 0 {
            return Ok(Self(bytes));
        }

        buffer.clear();
        buffer.reserve(ALIGN + bytes.len());
        buffer.extend_from_slice(&[0; ALIGN]);

        // SAFETY: just extended
        let ptr = unsafe { bytes.get_unchecked(0) as *const u8 };
        let offset = ptr.align_offset(ALIGN);

        if offset == usize::MAX {
            return Err(AlignedBytesError {
                reason: format!("AlignedBytes: impossible to align({ALIGN}) ptr({ptr:p})"),
                buffer: None,
            });
        }

        assert!(offset < ALIGN);

        let head = ALIGN - offset;

        // Remove extra zeroes
        buffer.drain(head..);
        buffer.extend_from_slice(bytes);

        // Check to be sure
        // SAFETY: just inserted
        let ptr = unsafe { bytes.get_unchecked(head) as *const u8 };
        let offset = ptr.align_offset(ALIGN);

        if offset != 0 {
            return Err(AlignedBytesError {
                reason: format!("AlignedBytes: vector reallocated, offset({offset})"),
                buffer: None,
            });
        }

        Ok(Self(&buffer[head..(head + bytes.len())]))
    }

    pub fn as_slice(&self) -> &'a [u8] {
        &self.0
    }
}

impl<const ALIGN: usize> OwnedAlignedBytes<ALIGN> {
    pub fn empty() -> Self {
        Self {
            buffer: Vec::new(),
            head: 0,
            len: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        assert!(ALIGN > 0, "align should be at least 1");

        Self {
            buffer: Vec::with_capacity(capacity + ALIGN - 1),
            head: 0,
            len: 0,
        }
    }

    /**
        Returns mut slice for specified length. After write this slice will be aligned.
        This function is unsafe, because slice should be written and after that `.set_length()` is
        called on the buffer.
    */
    pub unsafe fn write_slice(
        &mut self,
        len: usize,
    ) -> Result<WriteAlignedBytes<ALIGN>, AlignedBytesError> {
        let head = Self::calculate_head_for_buffer(&self.buffer)?;
        let end_len = head + len;

        self.buffer.clear();
        self.buffer.reserve(end_len);

        // Fill prefix with zeroes
        self.buffer.extend_from_slice(&(&[0; ALIGN])[0..head]);

        Ok(WriteAlignedBytes {
            aligned: self,
            head,
            len,
            is_cancelled: false,
        })
    }

    pub fn new(
        mut buffer: Vec<u8>,
        mut head: usize,
        len: usize,
    ) -> Result<Self, AlignedBytesError> {
        assert!(ALIGN.is_power_of_two(), "Invalid alignment {ALIGN}");

        let head_ptr = match buffer.get(head) {
            None => {
                return Err(AlignedBytesError {
                    reason: String::from("OwnedAlignedBytes: head is not inside bytes"),
                    buffer: Some(buffer),
                });
            }
            Some(x) => x as *const u8,
        };

        let last = head + len;
        if len == 0 || last == 0 {
            // Both head and len is 0, no need to align non-existing slice
            return Ok(Self { buffer, head, len });
        }
        if buffer.len() < head + len {
            return Err(AlignedBytesError {
                reason: String::from("OwnedAlignedBytes: len is outside of vector"),
                buffer: Some(buffer),
            });
        }

        let offset = head_ptr.align_offset(ALIGN);

        if offset == 0 {
            return Ok(Self { buffer, head, len });
        }

        if offset == usize::MAX {
            return Err(AlignedBytesError {
                reason: format!(
                    "OwnedAlignedBytes: impossible to align({ALIGN}) ptr({head_ptr:p})"
                ),
                buffer: Some(buffer),
            });
        }

        let forward_offset = offset;
        let backward_offset = ALIGN - offset;
        assert!(backward_offset <= ALIGN);

        if head >= backward_offset {
            // Prefer to move bytes back, not forward (which may require reallocation)
            buffer.copy_within(head..(head + len), head - backward_offset);
            head -= backward_offset;
        } else {
            buffer.reserve(forward_offset);
            buffer.copy_within(head..(head + len), head + forward_offset);
            head += forward_offset;
        }

        Ok(Self { buffer, head, len })
    }

    fn calculate_head_for_buffer(buffer: &Vec<u8>) -> Result<usize, AlignedBytesError> {
        let head_ptr = unsafe {
            let ptr = buffer.get_unchecked(0) as *const u8;
            // Pointing to potential head
            ptr.add(ALIGN)
        };

        let offset = head_ptr.align_offset(ALIGN);

        if offset == usize::MAX {
            return Err(AlignedBytesError {
                reason: format!(
                    "OwnedAlignedBytes: impossible to align({ALIGN}) ptr({head_ptr:p})"
                ),
                buffer: None,
            });
        }

        assert!(offset <= ALIGN);

        let backward_offset = ALIGN - offset;

        Ok(backward_offset)
    }

    pub fn copy_slice(mut buffer: Vec<u8>, bytes: &[u8]) -> Result<Self, AlignedBytesError> {
        buffer.clear();
        buffer.reserve(ALIGN + bytes.len());
        // Prefix to be able to move without second resize
        buffer.extend_from_slice(&[0; ALIGN]);

        let backward_offset = match Self::calculate_head_for_buffer(&buffer) {
            Ok(x) => x,
            Err(err) => {
                return Err(AlignedBytesError {
                    reason: err.reason,
                    buffer: Some(buffer),
                });
            }
        };

        let head = if backward_offset == ALIGN {
            // No need to align
            buffer.extend_from_slice(bytes);
            ALIGN
        } else {
            let head = ALIGN - backward_offset;
            // Remove last `backward_offset` bytes
            _ = buffer.drain(head..);

            buffer.extend_from_slice(bytes);

            head
        };

        let len = bytes.len();

        // This should be noop, but we need to check, maybe vector was reallocated
        Self::new(buffer, head, len)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.head..(self.head + self.len)]
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.buffer
    }

    pub fn into_raw_parts(self) -> (Vec<u8>, usize, usize) {
        let Self {
            buffer: bytes,
            head,
            len,
        } = self;
        (bytes, head, len)
    }

    pub fn as_ref(&self) -> AlignedBytes<'_, ALIGN> {
        AlignedBytes(&self.as_slice())
    }
}

pub struct WriteAlignedBytes<'a, const ALIGN: usize> {
    aligned: &'a mut OwnedAlignedBytes<ALIGN>,
    head: usize,
    len: usize,
    is_cancelled: bool,
}

impl<'a, const ALIGN: usize> WriteAlignedBytes<'a, ALIGN> {
    pub unsafe fn as_mut(&mut self) -> *mut u8 {
        self.aligned.buffer.get_unchecked_mut(self.head) as *mut u8
    }

    pub fn cancel(mut self) {
        self.is_cancelled = true;
    }
}

impl<'a, const ALIGN: usize> Drop for WriteAlignedBytes<'a, ALIGN> {
    fn drop(&mut self) {
        if self.is_cancelled {
            return;
        }

        // SAFETY: `write_slice()` constructor of this struct is unsafe
        unsafe {
            self.aligned.buffer.set_len(self.head + self.len);
        }
    }
}
