use core::ptr::slice_from_raw_parts;
use core::str::from_utf8_unchecked;
use core::{ptr, slice};

#[derive(Copy, Clone)]
#[repr(C)]
pub struct RegexCapture {
    capture: *const u8,
    capture_len: i32,
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
}

pub struct Regex {
    ptr: i32,
}

impl Regex {
    pub fn new(regex: &str) -> Self {
        let regex_ptr = regex.as_ptr();
        let regex_len = regex.len();
        let ptr = unsafe { new(regex_ptr, regex_len as i32) };

        if ptr < 0 {
            panic!("Regex::new returned negative pointer");
        }

        Self { ptr }
    }

    pub fn alloc_captures<const MAX_CAPTURES: usize>() -> [RegexCapture; MAX_CAPTURES] {
        [RegexCapture {
            capture: ptr::null(),
            capture_len: 0,
        }; MAX_CAPTURES]
    }

    pub fn captures<'a, const MAX_CAPTURES: usize>(
        &self,
        s: &'a str,
        mem: &'a mut [RegexCapture; MAX_CAPTURES],
    ) -> Option<Captures<'a>> {
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
            mem: &mem[0..(captures_count as usize)],
        })
    }
}

pub struct Captures<'a> {
    mem: &'a [RegexCapture],
}

impl Captures<'_> {
    pub fn get(&self, index: usize) -> Option<&str> {
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
