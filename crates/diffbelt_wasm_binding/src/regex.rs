use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use core::{ptr, slice};
use core::marker::PhantomData;
use core::str::from_utf8_unchecked;

use thiserror_no_std::Error;

use crate::{BytesVecFull, debug_print_string};
use crate::ptr::{NativePtrImpl, PtrImpl};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RegexCapture {
    capture: *const u8,
    capture_len: i32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct ReplaceResult<P: PtrImpl = NativePtrImpl> {
    pub is_same: i32,
    pub s: BytesVecFull<P>,
}

#[link(wasm_import_module = "Regex")]
extern "C" {
    fn new(regex: *const u8, regex_size: i32) -> i32;
    fn free(ptr: i32) -> ();
    fn captures(
        ptr: i32,
        s: *const u8,
        s_size: i32,
        captures_ptr: *mut RegexCapture,
        captures_max_count: i32,
    ) -> i32;
    fn replace_one(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
        replace_result: *mut ReplaceResult,
    ) -> ();
    fn replace_all(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
        replace_result: *mut ReplaceResult,
    ) -> ();
}

struct ReplaceOneImpl;
struct ReplaceAllImpl;

trait ReplaceMode {
    const REPLACE_FN: unsafe extern "C" fn(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
        replace_result: *mut ReplaceResult,
    ) -> ();
}

impl ReplaceMode for ReplaceOneImpl {
    const REPLACE_FN: unsafe extern "C" fn(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
        replace_result: *mut ReplaceResult,
    ) -> () = replace_one;
}

impl ReplaceMode for ReplaceAllImpl {
    const REPLACE_FN: unsafe extern "C" fn(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
        replace_result: *mut ReplaceResult,
    ) -> () = replace_all;
}

pub struct Regex {
    ptr: i32,
}

#[derive(Error, Debug)]
pub enum RegexError {
    CreationUnknown,
    ReplaceUnknown,
}

impl Regex {
    pub fn new(regex: &str) -> Result<Self, RegexError> {
        let regex_ptr = regex.as_ptr();
        let regex_len = regex.len();
        let ptr = unsafe { new(regex_ptr, regex_len as i32) };

        if ptr < 0 {
            return Err(RegexError::CreationUnknown);
        }

        Ok(Self { ptr })
    }

    pub fn alloc_captures<const MAX_CAPTURES: usize>() -> [RegexCapture; MAX_CAPTURES] {
        [RegexCapture {
            capture: ptr::null(),
            capture_len: 0,
        }; MAX_CAPTURES]
    }

    pub fn captures<'s, 'mem, const MAX_CAPTURES: usize>(
        &'mem self,
        s: &'s str,
        mem: &'mem mut [RegexCapture; MAX_CAPTURES],
    ) -> Option<Captures<'s, 'mem>> {
        let s_ptr = s.as_ptr();
        let s_len = s.len();
        let captures_count = unsafe {
            captures(
                self.ptr,
                s_ptr,
                s_len as i32,
                &mut mem[0] as *mut RegexCapture,
                MAX_CAPTURES as i32,
            )
        };

        if captures_count <= 0 {
            return None;
        }

        Some(Captures {
            s: PhantomData::default(),
            mem: &mem[0..(captures_count as usize)],
        })
    }

    fn replace_inner<'a, Mode: ReplaceMode>(
        &self,
        source: &'a str,
        target: &str,
    ) -> Result<Cow<'a, str>, RegexError> {
        let source_ptr = source.as_ptr();
        let source_len = source.len();
        let target_ptr = target.as_ptr();
        let target_len = target.len();

        let mut replace_result = ReplaceResult {
            is_same: 0,
            s: BytesVecFull::null(),
        };

        unsafe {
            Mode::REPLACE_FN(
                self.ptr,
                source_ptr,
                source_len as i32,
                target_ptr,
                target_len as i32,
                &mut replace_result as *mut ReplaceResult,
            )
        };

        let ReplaceResult { is_same, s } = replace_result;

        if is_same == 1 {
            return Ok(Cow::Borrowed(source));
        }

        let data = unsafe { s.into_vec() };
        let data = unsafe { String::from_utf8_unchecked(data) };

        Ok(Cow::Owned(data))
    }

    pub fn replace_one<'a>(
        &self,
        source: &'a str,
        target: &str,
    ) -> Result<Cow<'a, str>, RegexError> {
        self.replace_inner::<ReplaceOneImpl>(source, target)
    }

    pub fn replace_all<'a>(
        &self,
        source: &'a str,
        target: &str,
    ) -> Result<Cow<'a, str>, RegexError> {
        self.replace_inner::<ReplaceAllImpl>(source, target)
    }
}

pub struct Captures<'s, 'mem> {
    s: PhantomData<&'s str>,
    mem: &'mem [RegexCapture],
}

impl<'s, 'mem> Captures<'s, 'mem> {
    pub fn get(&self, index: usize) -> Option<&'s str> {
        let RegexCapture {
            capture,
            capture_len,
        } = self.mem.get(index)?;

        let s = unsafe {
            let slice = slice::from_raw_parts(*capture, *capture_len as usize);
            from_utf8_unchecked(slice)
        };

        Some(s)
    }
}

impl Drop for Regex {
    fn drop(&mut self) {
        unsafe {
            free(self.ptr);
        }
    }
}
