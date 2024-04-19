use crate::{YamlNode, YamlSerializationError};
use diffbelt_util::debug_print::debug_print;
use diffbelt_util_no_std::cast::{checked_usize_to_i32, u64_to_usize};
use std::ffi::CStr;
use std::fmt::Write;
use std::mem::MaybeUninit;
use std::ptr;
use std::ptr::{addr_of_mut, slice_from_raw_parts};
use std::str::from_utf8;
use unsafe_libyaml::{yaml_document_end_event_initialize, yaml_document_start_event_initialize, yaml_emitter_delete, yaml_emitter_emit, yaml_emitter_flush, yaml_emitter_initialize, yaml_emitter_open, yaml_emitter_set_output, yaml_event_delete, yaml_event_t, yaml_event_type_t, yaml_mark_t, yaml_scalar_event_initialize, yaml_scalar_style_t, yaml_version_directive_t};

struct Closure {
    ptr: *const libc::c_void,
    fun: unsafe fn(*const libc::c_void, *const [u8]) -> libc::c_int,
}

impl YamlNode {
    pub fn serialize(&self, output: &mut impl Write) -> Result<(), YamlSerializationError> {
        unsafe {
            let mut emitter = Box::pin(MaybeUninit::uninit());
            let emitter = emitter.as_mut_ptr();

            let result = yaml_emitter_initialize(emitter);
            if !result.ok {
                return Err(YamlSerializationError::EmitterInitializationFailed);
            }

            let result = yaml_emitter_open(emitter);
            if !result.ok {
                return Err(YamlSerializationError::EmitterOpenFailed);
            }

            let closure = |bytes: *const [u8]| -> libc::c_int {
                let bytes = &*bytes;
                let Ok(s) = from_utf8(bytes) else {
                    return 0;
                };

                let Ok(()) = output.write_str(s) else {
                    return 0;
                };

                1
            };
            let (ptr, fun) = dismantle(&closure);
            let mut closure = Closure { ptr, fun };
            let closure = &mut closure as *mut Closure as *mut libc::c_void;

            () = yaml_emitter_set_output(emitter, emitter_handler, closure);

            let mut event = MaybeUninit::<yaml_event_t>::uninit();
            let event = event.as_mut_ptr();

            let result = yaml_document_start_event_initialize(
                event,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
                true,
            );
            if !result.ok {
                return Err(YamlSerializationError::Unknown);
            }

            let result = yaml_emitter_emit(emitter, event);
            if !result.ok {
                return Err(YamlSerializationError::EmitterEmitFailed);
            }

            let test_str = "test string".as_bytes();

            let result = yaml_scalar_event_initialize(
                event,
                ptr::null(),
                ptr::null(),
                test_str.as_ptr(),
                test_str.len() as i32,
                true,
                false,
                yaml_scalar_style_t::YAML_PLAIN_SCALAR_STYLE,
            );
            if !result.ok {
                return Err(YamlSerializationError::Unknown);
            }

            let result = yaml_emitter_emit(emitter, event);
            if !result.ok {
                return Err(YamlSerializationError::EmitterEmitFailed);
            }

            let result = yaml_emitter_flush(emitter);
            if !result.ok {
                return Err(YamlSerializationError::EmitterFlushFailed);
            }

            yaml_emitter_delete(emitter);
        }

        Ok(())
    }
}

unsafe fn emitter_handler(
    data: *mut libc::c_void,
    buffer: *mut libc::c_uchar,
    size: u64,
) -> libc::c_int {
    let bytes = slice_from_raw_parts(buffer, u64_to_usize(size)) as *const [u8];

    let closure = &mut *(data as *mut Closure);

    (closure.fun)(closure.ptr, bytes)
}

fn dismantle<T, U, F: FnMut(T) -> U>(
    f: &F,
) -> (*const libc::c_void, unsafe fn(*const libc::c_void, T) -> U) {
    unsafe fn foo<T, U, F: FnMut(T) -> U>(f: *const libc::c_void, t: T) -> U {
        let f = &mut *(f as *mut F);
        f(t)
    }

    (f as *const F as *const libc::c_void, foo::<T, U, F>)
}
