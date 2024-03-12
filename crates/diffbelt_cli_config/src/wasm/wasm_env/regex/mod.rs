use bytemuck::Pod;
use std::borrow::Cow;
use std::cmp::min;
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use diffbelt_util::Wrap;
use diffbelt_util_no_std::cast::{
    try_positive_i32_to_usize, try_usize_to_i32, unchecked_i32_to_u32, unchecked_usize_to_i32,
    unchecked_usize_to_u32, usize_to_u64,
};
use diffbelt_wasm_binding::ptr::bytes::BytesVecRawParts;
use diffbelt_wasm_binding::ReplaceResult;
use regex::Regex;
use wasmtime::{AsContext, AsContextMut, Caller, Linker, Memory, Store};

use crate::wasm::memory::Allocation;
use crate::wasm::types::{
    BytesVecFullTrait, WasmPtr, WasmPtrImpl, WasmPtrToByte, WasmPtrToBytesSlice, WasmReplaceResult,
};
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};

pub struct WasmRegex {
    regex: Regex,
}

pub struct RegexEnv {
    error: Arc<Mutex<Option<WasmError>>>,
    memory: Arc<Mutex<Option<Memory>>>,
    allocation: Arc<Mutex<Option<Allocation>>>,
    regexps: Arc<Mutex<Vec<WasmRegex>>>,
    free_regexps: Arc<Mutex<VecDeque<i32>>>,
}

impl WasmEnv {
    pub fn register_regex_wasm_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) {
        {
            *store.data_mut().regex = Some(RegexEnv {
                error: self.error.clone(),
                memory: self.memory.clone(),
                allocation: self.allocation.clone(),
                regexps: Wrap::wrap(Vec::new()),
                free_regexps: Wrap::wrap(VecDeque::new()),
            });
        }

        fn regex_new(mut caller: Caller<'_, WasmStoreData>, s: WasmPtrToByte, s_size: i32) -> i32 {
            let RegexEnv {
                error,
                memory,
                regexps,
                ..
            } = caller.data().regex.as_ref().expect("RegexEnv");

            let result = (|| {
                let mut memory = memory.lock().expect("lock");
                let memory = memory.as_ref().expect("no memory");

                let s = ptr_to_utf8(caller.as_context(), memory, s, s_size)?;
                let s = s.as_str().unwrap();

                let regex = Regex::new(s).map_err(WasmError::Regex)?;

                let index = try_usize_to_i32(regexps.len()).ok_or_else(|| {
                    WasmError::Unspecified("More than i32::MAX regexps".to_string())
                })?;

                let mut regexps = regexps.lock().expect("lock");
                regexps.push(WasmRegex { regex });

                Ok::<_, WasmError>(index)
            })();

            let Some(index) = WasmEnv::handle_error(&caller, result) else {
                return -1;
            };

            index
        }

        fn regex_free(mut caller: Caller<'_, WasmStoreData>, ptr: i32) {
            let RegexEnv { free_regexps, .. } = caller.data().regex.as_ref().expect("RegexEnv");

            let mut free_regexps = free_regexps.lock().expect("lock");
            free_regexps.push_back(ptr);
        }

        #[derive(Pod, Copy, Clone)]
        #[repr(C)]
        struct RegexCapture {
            capture: WasmPtrToByte,
            capture_len: i32,
        }

        fn regex_captures(
            mut caller: Caller<'_, WasmStoreData>,
            index: i32,
            s_ptr: WasmPtr<u8>,
            s_size: i32,
            captures_ptr: WasmPtr<RegexCapture>,
            max_captures_count: i32,
        ) -> i32 {
            let RegexEnv {
                memory, regexps, ..
            } = caller.data().regex.as_ref().expect("RegexEnv");

            let result = (|| {
                let index = try_positive_i32_to_usize(index).ok_or_else(|| {
                    WasmError::Unspecified(format!("Tried to get regexp at {index}"))
                })?;
                let max_captures_count =
                    try_positive_i32_to_usize(max_captures_count).ok_or_else(|| {
                        WasmError::Unspecified(format!("max_captures_count: {max_captures_count}"))
                    })?;
                let max_captures_count_u32 = unchecked_usize_to_u32(max_captures_count);

                let regexps = regexps.lock().expect("lock");
                let regexps = regexps.deref();

                let regex = regexps.get(index).ok_or_else(|| {
                    WasmError::Unspecified(format!(
                        "Tried to get regexp at {index}, there is only {} of them",
                        regexps.len()
                    ))
                })?;

                let mut memory_lock = memory.lock().expect("lock");
                let memory = memory_lock.as_ref().expect("no memory");

                let s = ptr_to_utf8(caller.as_context(), memory, s_ptr, s_size)?;
                let s = s.as_str()?;

                let Some(captures) = regex.regex.captures(s) else {
                    return Ok(0);
                };

                let captures_count = captures.len();
                let captures_count = min(captures_count, max_captures_count);

                let captures_slice = captures_ptr.slice()?;

                let memory_bytes = memory_lock.as_mut().expect("no_memory");
                let memory_bytes = memory_bytes.data_mut(&caller);

                for (i, capture) in captures.iter().enumerate() {
                    if i >= captures_count {
                        break;
                    }

                    let Some(m) = capture else {
                        () = captures_slice.write_at(
                            memory_bytes,
                            i,
                            RegexCapture {
                                capture: WasmPtr::null(),
                                capture_len: 0,
                            },
                        )?;
                        continue;
                    };

                    let capture_ptr = s_ptr
                        .clone()
                        .add_offset(unchecked_usize_to_u32(m.start()))?;
                    let capture_len = m.len();

                    () = captures_slice.write_at(
                        memory_bytes,
                        i,
                        RegexCapture {
                            capture: capture_ptr,
                            capture_len: try_usize_to_i32(capture_len).ok_or_else(|| {
                                WasmError::Unspecified(format!("Regex capture size: {capture_len}"))
                            })?,
                        },
                    )?;
                }

                Ok::<_, WasmError>(unchecked_usize_to_i32(captures_count))
            })();

            let Some(captures_count) = WasmEnv::handle_error(&caller, result) else {
                return -1;
            };

            captures_count
        }

        struct ReplaceOneImpl;
        struct ReplaceAllImpl;

        trait ReplaceImpl {
            fn do_replace<'a>(regex: &Regex, source: &'a str, target: &str) -> Cow<'a, str>;
        }

        impl ReplaceImpl for ReplaceOneImpl {
            fn do_replace<'a>(regex: &Regex, source: &'a str, target: &str) -> Cow<'a, str> {
                regex.replace(source, target)
            }
        }

        impl ReplaceImpl for ReplaceAllImpl {
            fn do_replace<'a>(regex: &Regex, source: &'a str, target: &str) -> Cow<'a, str> {
                regex.replace_all(source, target)
            }
        }

        fn regex_replace<Mode: ReplaceImpl>(
            mut caller: Caller<'_, WasmStoreData>,
            ptr: i32,
            source_ptr: WasmPtr<u8>,
            source_len: i32,
            target_ptr: WasmPtr<u8>,
            target_len: i32,
            replace_result_ptr: WasmPtr<WasmReplaceResult>,
        ) -> () {
            let RegexEnv {
                error,
                memory,
                allocation,
                regexps,
                ..
            } = caller.data().regex.as_ref().expect("RegexEnv");

            let mut memory = memory.lock().expect("lock");
            let memory = memory.as_mut().expect("no memory");

            let result = (|| {
                let result = {
                    let ptr = try_positive_i32_to_usize(ptr).ok_or_else(|| {
                        WasmError::Unspecified(format!("Tried to get regexp at {ptr}"))
                    })?;

                    let regex = regexps.get(ptr).ok_or_else(|| {
                        WasmError::Unspecified(format!(
                            "Tried to get regexp at {ptr}, there is only {} of them",
                            regexps.len()
                        ))
                    })?;

                    let source = ptr_to_utf8(caller.as_context(), memory, source_ptr, source_len)?;
                    let source = source.as_str()?;

                    let target = ptr_to_utf8(caller.as_context(), memory, target_ptr, target_len)?;
                    let target = target.as_str()?;

                    let result = Mode::do_replace(&regex.regex, source, target);

                    // TODO: check that `result` is Borrowed and that it has same address as `source`,
                    //       calculate start/end offsets and return partial
                    if result.as_ref() == source {
                        return Ok::<_, WasmError>(ReplaceResult::<WasmPtrImpl> {
                            is_same: 1,
                            s: BytesVecFullTrait::null(),
                        });
                    }

                    result.into_owned()
                };

                let allocation = allocation.lock().map_err(|_| WasmError::MutexPoisoned)?;
                let allocation = allocation.as_ref().ok_or_else(|| WasmError::NoMemory)?;

                let result = result.as_bytes();

                let result_bytes_len_i32 = try_usize_to_i32(result.len()).ok_or_else(|| {
                    WasmError::Unspecified(format!(
                        "regex_replace result too big: {}",
                        result.len()
                    ))
                })?;

                let vec_ptr = allocation.alloc.call(&mut caller, result_bytes_len_i32)?;

                {
                    let vec_slice = vec_ptr.slice()?;
                    () = vec_slice.write_slice(memory, result)?;
                }

                Ok(ReplaceResult::<WasmPtrImpl> {
                    is_same: 0,
                    s: BytesVecRawParts {
                        ptr: vec_ptr.into(),
                        len: result_bytes_len_i32,
                        capacity: result_bytes_len_i32,
                    },
                })
            })();

            let Some(result) = WasmEnv::handle_error(error, result) else {
                return ();
            };

            let result = (|| {
                () = replace_result_ptr.write(&view, WasmReplaceResult(result))?;

                Ok::<(), WasmError>(())
            })();

            () = WasmEnv::handle_error(error, result).unwrap_or(());
        }

        linker.func_wrap("Regex", "new", regex_new)?;
        linker.func_wrap("Regex", "free", regex_free)?;
        linker.func_wrap("Regex", "captures", regex_captures)?;
        linker.func_wrap("Regex", "replace_one", regex_replace::<ReplaceOneImpl>)?;
        linker.func_wrap("Regex", "replace_all", regex_replace::<ReplaceAllImpl>)?;
    }
}
