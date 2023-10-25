use crate::debug_print::debug_print_string;
use crate::global_allocator::from_raw_parts;
use alloc::borrow::Cow;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use core::marker::PhantomData;
use core::ptr::slice_from_raw_parts;
use core::str::from_utf8_unchecked;
use core::{ptr, slice};
use thiserror_no_std::Error;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RegexCapture {
    capture: *const u8,
    capture_len: i32,
}

type ReplaceResult = (i32, i32, i32, i32);

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
    ) -> ReplaceResult;
    fn replace_all(
        ptr: i32,
        source: *const u8,
        source_len: i32,
        target: *const u8,
        target_len: i32,
    ) -> ReplaceResult;
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
    ) -> ReplaceResult;
}

impl ReplaceMode for ReplaceOneImpl {
    const REPLACE_FN: unsafe extern "C" fn(i32, *const u8, i32, *const u8, i32) -> ReplaceResult =
        replace_one;
}

impl ReplaceMode for ReplaceAllImpl {
    const REPLACE_FN: unsafe extern "C" fn(i32, *const u8, i32, *const u8, i32) -> ReplaceResult =
        replace_all;
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

        let (is_same, s, s_len, s_capacity) = unsafe {
            Mode::REPLACE_FN(
                self.ptr,
                source_ptr,
                source_len as i32,
                target_ptr,
                target_len as i32,
            )
        };

        let s = s as *mut u8;

        debug_print_string(format!("--- {is_same} {s:p} {s_len} {s_capacity}"));

        if is_same == 1 {
            return Ok(Cow::Borrowed(source));
        }

        if s_len < 0 || s_capacity < 0 {
            return Err(RegexError::ReplaceUnknown);
        }

        let data = unsafe { from_raw_parts(s, s_len, s_capacity) };
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
