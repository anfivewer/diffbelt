#![no_std]

pub mod pool;
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
    pub fn align_in_vec(
        buffer: &'a mut Vec<u8>,
        head: usize,
        len: usize,
    ) -> Result<Self, AlignedBytesError> {
        let (s, _head) = Self::align_in_vec_inner(buffer, 0, head, len)?;
        Ok(s)
    }

    pub fn align_in_prefixed_vec(
        buffer: &'a mut Vec<u8>,
        prefix: usize,
        head: usize,
        len: usize,
    ) -> Result<Self, AlignedBytesError> {
        let (s, _head) = Self::align_in_vec_inner(buffer, prefix, head, len)?;
        Ok(s)
    }

    fn align_in_vec_inner(
        buffer: &'a mut Vec<u8>,
        prefix: usize,
        mut head: usize,
        len: usize,
    ) -> Result<(Self, usize), AlignedBytesError> {
        if len == 0 {
            return Ok((Self(&[]), 0));
        }

        assert!(head >= prefix, "head should be after prefix");

        if buffer.len() < head + len {
            return Err(AlignedBytesError {
                reason: String::from("AlignedBytes: len is outside of vector"),
                buffer: None,
            });
        }

        // SAFETY: just checked lengths
        let head_ptr = unsafe { buffer.as_ptr().add(head) };

        let offset = head_ptr.align_offset(ALIGN);

        if offset == 0 {
            return Ok((Self(&buffer[head..(head + len)]), head));
        }

        if offset == usize::MAX {
            return Err(AlignedBytesError {
                reason: format!("AlignedBytes: impossible to align({ALIGN}) ptr({head_ptr:p})"),
                buffer: None,
            });
        }

        let start_offset = buffer.as_ptr().wrapping_add(prefix).align_offset(ALIGN);

        if head >= start_offset {
            // Prefer to move bytes back, not forward (which may require reallocation)
            buffer.copy_within(head..(head + len), start_offset);
            head = start_offset;
        } else {
            if buffer.len() < prefix + len + ALIGN {
                let need_extend_for = prefix + len + ALIGN - buffer.len();
                buffer.reserve(need_extend_for);
                for _ in 0..need_extend_for {
                    buffer.push(0);
                }
            }

            // We are reserved extra space, buffer maybe is reallocated
            let start_ptr = buffer.as_ptr();
            // SAFETY: checked in the start
            let head_ptr = unsafe { start_ptr.add(head) };
            // SAFETY: we are allocated up to prefix and more
            let prefixed_ptr = unsafe { start_ptr.add(prefix) };

            let head_align_offset = head_ptr.align_offset(ALIGN);
            if head_align_offset == 0 {
                // We are lucky, after realloc all is aligned :)
                return Ok((Self(&buffer[head..(head + len)]), head));
            }

            let offset = prefixed_ptr.align_offset(ALIGN);

            buffer.copy_within(head..(head + len), prefix + offset);
            head = offset;
        }

        Ok((Self(&buffer[head..(head + len)]), head))
    }

    pub fn ensure_alignment(bytes: &'a [u8]) -> Result<Self, AlignedBytesError> {
        let offset = bytes.as_ptr().align_offset(ALIGN);

        if offset != 0 {
            return Err(AlignedBytesError {
                reason: format!("not aligned, offset: {offset}"),
                buffer: None,
            });
        }

        Ok(Self(bytes))
    }

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

    pub fn new(mut buffer: Vec<u8>, head: usize, len: usize) -> Result<Self, AlignedBytesError> {
        let (_aligned, head) =
            AlignedBytes::<ALIGN>::align_in_vec_inner(&mut buffer, 0, head, len)?;

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
