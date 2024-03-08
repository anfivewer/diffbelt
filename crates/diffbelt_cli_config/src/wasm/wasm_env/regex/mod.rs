use std::borrow::Cow;
use std::cmp::min;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use diffbelt_util_no_std::cast::{
    try_positive_i32_to_usize, try_usize_to_i32, unchecked_i32_to_u32, unchecked_usize_to_i32,
    unchecked_usize_to_u32, usize_to_u64,
};
use diffbelt_wasm_binding::ptr::bytes::BytesVecRawParts;
use diffbelt_wasm_binding::ReplaceResult;
use regex::Regex;

use crate::wasm::memory::Allocation;
use crate::wasm::types::{BytesVecFullTrait, WasmPtrImpl, WasmReplaceResult};
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::WasmError;

pub struct WasmRegex {
    regex: Regex,
}

impl WasmEnv {
    pub fn register_regex_wasm_imports(&self, store: &mut Store, imports: &mut Imports) {
        struct RegexEnv {
            error: Arc<Mutex<Option<WasmError>>>,
            memory: Arc<Mutex<Option<Memory>>>,
            allocation: Arc<Mutex<Option<Allocation>>>,
            regexps: Vec<WasmRegex>,
            free_regexps: VecDeque<i32>,
        }

        let env = FunctionEnv::new(
            store,
            RegexEnv {
                error: self.error.clone(),
                memory: self.memory.clone(),
                allocation: self.allocation.clone(),
                regexps: Vec::new(),
                free_regexps: VecDeque::new(),
            },
        );

        fn regex_new(mut env: FunctionEnvMut<RegexEnv>, s: WasmPtr<u8>, s_size: i32) -> i32 {
            let (env, store) = env.data_and_store_mut();
            let RegexEnv {
                error,
                memory,
                regexps,
                ..
            } = env;

            let result = (|| {
                let view = WasmEnv::memory_view(memory, &store)?;

                let s = ptr_to_utf8(&view, s, s_size).unwrap();
                let s = s.as_str().unwrap();

                let regex = Regex::new(s).map_err(WasmError::Regex)?;

                let index = try_usize_to_i32(regexps.len()).ok_or_else(|| {
                    WasmError::Unspecified("More than i32::MAX regexps".to_string())
                })?;

                regexps.push(WasmRegex { regex });

                Ok::<_, WasmError>(index)
            })();

            let Some(index) = WasmEnv::handle_error(error, result) else {
                return -1;
            };

            index
        }

        fn regex_free(mut env: FunctionEnvMut<RegexEnv>, ptr: i32) {
            let env = env.data_mut();
            let RegexEnv { free_regexps, .. } = env;

            free_regexps.push_back(ptr);
        }

        #[derive(Copy, Clone, ValueType)]
        #[repr(C)]
        struct RegexCapture {
            capture: WasmPtr<u8>,
            capture_len: i32,
        }

        fn regex_captures(
            mut env: FunctionEnvMut<RegexEnv>,
            ptr: i32,
            s_ptr: WasmPtr<u8>,
            s_size: i32,
            captures_ptr: WasmPtr<RegexCapture>,
            max_captures_count: i32,
        ) -> i32 {
            let (env, store) = env.data_and_store_mut();
            let RegexEnv {
                error,
                memory,
                regexps,
                ..
            } = env;

            let result = (|| {
                let view = WasmEnv::memory_view(memory, &store)?;

                let ptr = try_positive_i32_to_usize(ptr).ok_or_else(|| {
                    WasmError::Unspecified(format!("Tried to get regexp at {ptr}"))
                })?;
                let max_captures_count =
                    try_positive_i32_to_usize(max_captures_count).ok_or_else(|| {
                        WasmError::Unspecified(format!("max_captures_count: {max_captures_count}"))
                    })?;
                let max_captures_count_u32 = unchecked_usize_to_u32(max_captures_count);

                let regex = regexps.get(ptr).ok_or_else(|| {
                    WasmError::Unspecified(format!(
                        "Tried to get regexp at {ptr}, there is only {} of them",
                        regexps.len()
                    ))
                })?;

                let s = ptr_to_utf8(&view, s_ptr, s_size)?;
                let s = s.as_str()?;

                let Some(captures) = regex.regex.captures(s) else {
                    return Ok(0);
                };

                let captures_count = captures.len();
                let captures_count = min(captures_count, max_captures_count);

                let captures_slice = captures_ptr.slice(&view, max_captures_count_u32)?;

                for (i, capture) in captures.iter().enumerate() {
                    if i >= captures_count {
                        break;
                    }

                    let Some(m) = capture else {
                        () = captures_slice.write(
                            usize_to_u64(i),
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

                    () = captures_slice.write(
                        usize_to_u64(i),
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

            let Some(captures_count) = WasmEnv::handle_error(error, result) else {
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
            mut env: FunctionEnvMut<RegexEnv>,
            ptr: i32,
            source_ptr: WasmPtr<u8>,
            source_len: i32,
            target_ptr: WasmPtr<u8>,
            target_len: i32,
            replace_result_ptr: WasmPtr<WasmReplaceResult>,
        ) -> () {
            let (env, mut store) = env.data_and_store_mut();
            let RegexEnv {
                error,
                memory,
                allocation,
                regexps,
                ..
            } = env;

            let result = (|| {
                let result = {
                    let view = WasmEnv::memory_view(memory, &store)?;

                    let ptr = try_positive_i32_to_usize(ptr).ok_or_else(|| {
                        WasmError::Unspecified(format!("Tried to get regexp at {ptr}"))
                    })?;

                    let regex = regexps.get(ptr).ok_or_else(|| {
                        WasmError::Unspecified(format!(
                            "Tried to get regexp at {ptr}, there is only {} of them",
                            regexps.len()
                        ))
                    })?;

                    let source = ptr_to_utf8(&view, source_ptr, source_len)?;
                    let source = source.as_str()?;

                    let target = ptr_to_utf8(&view, target_ptr, target_len)?;
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

                let vec_ptr = allocation.alloc.call(&mut store, result_bytes_len_i32)?;

                {
                    let view = WasmEnv::memory_view(memory, &store)?;

                    let vec_slice =
                        vec_ptr.slice(&view, unchecked_i32_to_u32(result_bytes_len_i32))?;
                    () = vec_slice.write_slice(result)?;
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
                let view = WasmEnv::memory_view(memory, &store)?;

                () = replace_result_ptr.write(&view, WasmReplaceResult(result))?;

                Ok::<(), WasmError>(())
            })();

            () = WasmEnv::handle_error(error, result).unwrap_or(());
        }

        imports.define(
            "Regex",
            "new",
            Function::new_typed_with_env(store, &env, regex_new),
        );
        imports.define(
            "Regex",
            "free",
            Function::new_typed_with_env(store, &env, regex_free),
        );
        imports.define(
            "Regex",
            "captures",
            Function::new_typed_with_env(store, &env, regex_captures),
        );
        imports.define(
            "Regex",
            "replace_one",
            Function::new_typed_with_env(store, &env, regex_replace::<ReplaceOneImpl>),
        );
        imports.define(
            "Regex",
            "replace_all",
            Function::new_typed_with_env(store, &env, regex_replace::<ReplaceAllImpl>),
        );
    }
}
