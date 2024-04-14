use crate::{YamlNode, YamlSerializationError};
use std::fmt::Write;
use std::mem::MaybeUninit;
use std::ptr;
use unsafe_libyaml::{
    yaml_emitter_delete, yaml_emitter_initialize, yaml_emitter_open, yaml_emitter_set_output,
};

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

            () = yaml_emitter_set_output(emitter, |data, buffer, size| 0, ptr::null_mut());

            yaml_emitter_delete(emitter);
        }

        todo!()
    }
}

unsafe fn emitter_handler(data: *mut libc::c_void, buffer: *mut libc::c_uchar, size: libc::size_t) -> libc::c_int {
    todo!()
}