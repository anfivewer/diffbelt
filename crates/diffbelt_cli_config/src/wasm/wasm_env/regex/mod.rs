use std::cmp::min;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use regex::Regex;
use wasmer::{
    AsStoreRef, Function, FunctionEnv, FunctionEnvMut, Imports, Memory, MemoryView, Store,
    ValueType, WasmPtr,
};

use diffbelt_util::cast::{
    try_positive_i32_to_u32, try_positive_i32_to_usize, try_usize_to_i32, unchecked_usize_to_i32,
    unchecked_usize_to_u32, usize_to_u64,
};
use diffbelt_util::Wrap;

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
            regexps: Vec<WasmRegex>,
            free_regexps: VecDeque<i32>,
        }

        let env = FunctionEnv::new(
            store,
            RegexEnv {
                error: self.error.clone(),
                memory: self.memory.clone(),
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

                let s = ptr_to_utf8(&view, s_ptr, s_size).unwrap();
                let s = s.as_str().unwrap();

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
    }
}