use std::ffi::CStr;
use std::fmt::Write;
use std::mem::MaybeUninit;
use std::ptr;
use std::ptr::slice_from_raw_parts;
use std::str::from_utf8;

use unsafe_libyaml::{
    yaml_document_start_event_initialize, yaml_emitter_delete, yaml_emitter_emit,
    yaml_emitter_flush, yaml_emitter_initialize, yaml_emitter_open, yaml_emitter_set_output,
    yaml_emitter_t, yaml_event_t,
};

use diffbelt_util_no_std::cast::u64_to_usize;
use diffbelt_util_no_std::temporary_collection::vec::TempVecType;

use crate::serialization::mapping::emit_mapping;
use crate::serialization::scalar::emit_scalar;
use crate::serialization::sequence::emit_sequence;
use crate::{YamlNode, YamlNodeValue, YamlSerializationError};

mod mapping;
mod scalar;
mod sequence;

struct SerializationContext {
    emitter: *mut yaml_emitter_t,
    events: Vec<MaybeUninit<yaml_event_t>>,
}

impl SerializationContext {
    fn take_event(&mut self) -> *mut yaml_event_t {
        let item = MaybeUninit::uninit();
        self.events.push(item);
        self.events.last_mut().expect("just inserted").as_mut_ptr()
    }
}

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
            () = check_error(result.ok, emitter)?;

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
            () = check_error(result.ok, emitter)?;

            let result = yaml_emitter_emit(emitter, event);
            () = check_error(result.ok, emitter)?;

            let mut ctx = SerializationContext {
                emitter,
                events: Vec::with_capacity(64),
            };

            () = self.serialize_node(&mut ctx)?;

            let result = yaml_emitter_flush(emitter);
            () = check_error(result.ok, emitter)?;

            yaml_emitter_delete(emitter);
        }

        Ok(())
    }

    unsafe fn serialize_node(
        &self,
        ctx: &mut SerializationContext,
    ) -> Result<(), YamlSerializationError> {
        match &self.value {
            YamlNodeValue::Empty => {
                panic!("TODO: empty node")
            }
            YamlNodeValue::Scalar(scalar) => emit_scalar(ctx, scalar),
            YamlNodeValue::Sequence(sequence) => emit_sequence(ctx, sequence),
            YamlNodeValue::Mapping(mapping) => emit_mapping(ctx, mapping),
        }
    }
}

unsafe fn check_error(
    success: bool,
    emitter: *mut yaml_emitter_t,
) -> Result<(), YamlSerializationError> {
    if success {
        return Ok(());
    }

    let problem = (&*emitter).problem;

    if problem.is_null() {
        return Err(YamlSerializationError::Unknown);
    }

    let s = CStr::from_ptr(problem);
    let s = s.to_str()?;

    Err(YamlSerializationError::Unspecified(String::from(s)))
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
