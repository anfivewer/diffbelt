#![no_std]

#[cfg(test)]
mod tests;

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::{Debug, Formatter};

pub struct OwnedAlignedBytes<const ALIGN: usize> {
    buffer: Vec<u8>,
    head: usize,
    len: usize,
}

impl<const ALIGN: usize> Debug for OwnedAlignedBytes<ALIGN> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str("OwnedAlignedBytes(")?;
        f.write_fmt(format_args!("head:{} ", self.head))?;
        f.write_fmt(format_args!("len:{}", self.len))?;
        f.write_str(")")?;
        Ok(())
    }
}

impl<const ALIGN: usize> Default for OwnedAlignedBytes<ALIGN> {
    fn default() -> Self {
        Self::empty()
    }
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

        let ptr = bytes.as_ptr();
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

        let ptr = bytes.as_ptr();
        let offset = ptr.align_offset(ALIGN);

        buffer.extend_from_slice(&(&[0; ALIGN])[0..offset]);
        buffer.extend_from_slice(bytes);

        Ok(Self(&buffer[offset..(offset + bytes.len())]))
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
        self.buffer.clear();
        self.buffer.reserve(ALIGN + len);

        let head = Self::calculate_head_for_buffer(&self.buffer)?;

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
            if last + forward_offset >= buffer.len() {
                let need_extend_for = last + forward_offset - buffer.len() + 1;
                buffer.reserve(need_extend_for);
                for _ in 0..need_extend_for {
                    buffer.push(0);
                }
            }
            buffer.copy_within(head..(head + len), head + forward_offset);
            head += forward_offset;
        }

        Ok(Self { buffer, head, len })
    }

    fn calculate_head_for_buffer(buffer: &Vec<u8>) -> Result<usize, AlignedBytesError> {
        let head_ptr = buffer.as_ptr();
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

        Ok(offset)
    }

    pub fn copy_slice(mut buffer: Vec<u8>, bytes: &[u8]) -> Result<Self, AlignedBytesError> {
        buffer.clear();
        buffer.reserve(ALIGN + bytes.len());

        let head = match Self::calculate_head_for_buffer(&buffer) {
            Ok(x) => x,
            Err(err) => {
                return Err(AlignedBytesError {
                    reason: err.reason,
                    buffer: Some(buffer),
                });
            }
        };

        buffer.extend_from_slice(&(&[0; ALIGN])[0..head]);
        buffer.extend_from_slice(bytes);

        let len = bytes.len();

        // This should be noop, but we need to check, maybe vector was reallocated
        Self::new(buffer, head, len)
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[self.head..(self.head + self.len)]
    }

    pub fn into_underlying_vec(self) -> Vec<u8> {
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
        self.aligned.buffer.as_ptr().add(self.head) as *mut u8
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

        let new_len = self.head + self.len;

        // SAFETY: `write_slice()` constructor of this struct is unsafe
        unsafe {
            self.aligned.buffer.set_len(new_len);
            self.aligned.head = self.head;
            self.aligned.len = self.len;
        }
    }
}
