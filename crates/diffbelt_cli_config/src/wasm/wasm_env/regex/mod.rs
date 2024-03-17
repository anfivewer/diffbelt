use std::borrow::Cow;
use std::cmp::min;
use std::collections::VecDeque;
use std::future::Future;
use std::ops::DerefMut;

use diffbelt_util_no_std::cast::{
    try_positive_i32_to_usize, try_usize_to_i32, unchecked_usize_to_i32,
};
use diffbelt_util_no_std::temporary_collection::vec::{TempVecType, TemporaryVec};
use diffbelt_wasm_binding::ptr::bytes::BytesVecRawParts;
use diffbelt_wasm_binding::{RegexCapture, ReplaceResult};
use regex::Regex;
use wasmtime::{AsContext, AsContextMut, Caller, Linker, Store};

use crate::wasm::types::{
    BytesVecFullTrait, WasmPtr, WasmPtrImpl, WasmPtrToByte, WasmReplaceResult,
};
use crate::wasm::wasm_env::util::ptr_to_utf8;
use crate::wasm::wasm_env::WasmEnv;
use crate::wasm::{WasmError, WasmStoreData};

pub struct WasmRegex {
    regex: Regex,
}

type WasmRegexCapture = RegexCapture<WasmPtrImpl>;

struct WasmRegexCaptureTemp;
impl TempVecType for WasmRegexCaptureTemp {
    type Item<'a> = WasmRegexCapture;
}

pub struct RegexEnv {
    regexps: Vec<WasmRegex>,
    free_regexps: VecDeque<i32>,
    temp_captures_vec: TemporaryVec<WasmRegexCaptureTemp>,
}

impl WasmEnv {
    pub fn register_regex_wasm_imports(
        &self,
        store: &mut Store<WasmStoreData>,
        linker: &mut Linker<WasmStoreData>,
    ) -> Result<(), WasmError> {
        {
            let mut state = store.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            state.regex = Some(RegexEnv {
                regexps: Vec::new(),
                free_regexps: VecDeque::new(),
                temp_captures_vec: TemporaryVec::new(),
            });
        }

        fn regex_new(caller: Caller<'_, WasmStoreData>, s: WasmPtrToByte, s_size: i32) -> i32 {
            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();

            // TODO: use free_regexps

            let memory = state.memory.expect("no memory");

            let RegexEnv { regexps, .. } = state.regex.as_mut().expect("RegexEnv");

            let result = (|| {
                let s = ptr_to_utf8(caller.as_context(), memory, s, s_size)?;
                let s = s.as_str().unwrap();

                let regex = Regex::new(s).map_err(WasmError::Regex)?;

                let index = try_usize_to_i32(regexps.len()).ok_or_else(|| {
                    WasmError::Unspecified("More than i32::MAX regexps".to_string())
                })?;

                regexps.push(WasmRegex { regex });

                Ok::<_, WasmError>(index)
            })();

            let Some(index) = WasmEnv::handle_error(&caller.data().error, result) else {
                return -1;
            };

            index
        }

        fn regex_free(caller: Caller<'_, WasmStoreData>, ptr: i32) {
            let mut state = caller.data().inner.lock().expect("lock");
            let state = state.deref_mut();
            let RegexEnv { free_regexps, .. } = state.regex.as_mut().expect("RegexEnv");

            free_regexps.push_back(ptr);
        }

        fn regex_captures(
            mut caller: Caller<'_, WasmStoreData>,
            index: i32,
            s_ptr: WasmPtr<u8>,
            s_size: i32,
            captures_ptr: WasmPtr<WasmRegexCapture>,
            max_captures_count: i32,
        ) -> i32 {
            let state = caller.data().inner.clone();
            let mut state = state.lock().expect("lock");
            let state = state.deref_mut();
            let memory = state.memory.expect("no memory");
            let RegexEnv {
                regexps,
                temp_captures_vec,
                ..
            } = state.regex.as_mut().expect("RegexEnv");

            let result = (|| {
                let index = try_positive_i32_to_usize(index).ok_or_else(|| {
                    WasmError::Unspecified(format!("Tried to get regexp at {index}"))
                })?;
                let max_captures_count =
                    try_positive_i32_to_usize(max_captures_count).ok_or_else(|| {
                        WasmError::Unspecified(format!("max_captures_count: {max_captures_count}"))
                    })?;

                let regex = regexps.get(index).ok_or_else(|| {
                    WasmError::Unspecified(format!(
                        "Tried to get regexp at {index}, there is only {} of them",
                        regexps.len()
                    ))
                })?;

                let s = ptr_to_utf8(caller.as_context(), memory, s_ptr, s_size)?;
                let s = s.as_str()?;

                let Some(captures) = regex.regex.captures(s) else {
                    return Ok(0);
                };

                let captures_count = captures.len();
                let captures_count = min(captures_count, max_captures_count);

                let captures_slice = captures_ptr.slice()?;
                let mut captures_to_write_holder = temp_captures_vec.temp();
                let captures_to_write = captures_to_write_holder.as_mut();

                for (i, capture) in captures.iter().enumerate() {
                    if i >= captures_count {
                        break;
                    }

                    let Some(m) = capture else {
                        captures_to_write.push(WasmRegexCapture {
                            capture: WasmPtr::null(),
                            capture_len: 0,
                        });
                        continue;
                    };

                    let offset = try_usize_to_i32(m.start()).ok_or_else(|| {
                        WasmError::Unspecified(format!(
                            "regex_captures too big offset {}",
                            m.start()
                        ))
                    })?;
                    let capture_ptr = s_ptr.clone().add_offset(offset)?;
                    let capture_len = m.len();

                    captures_to_write.push(WasmRegexCapture {
                        capture: capture_ptr,
                        capture_len: try_usize_to_i32(capture_len).ok_or_else(|| {
                            WasmError::Unspecified(format!("Regex capture size: {capture_len}"))
                        })?,
                    });
                }

                let memory_bytes = memory.data_mut(caller.as_context_mut());

                for (i, capture) in captures_to_write.drain(..).enumerate() {
                    () = captures_slice.write_at(memory_bytes, i, capture)?;
                }

                Ok::<_, WasmError>(unchecked_usize_to_i32(captures_count))
            })();

            let Some(captures_count) = WasmEnv::handle_error(&caller.data().error, result) else {
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

        fn regex_replace<'a, Mode: ReplaceImpl + 'static>(
            caller: Caller<'a, WasmStoreData>,
            ptr: i32,
            source_ptr: WasmPtr<u8>,
            source_len: i32,
            target_ptr: WasmPtr<u8>,
            target_len: i32,
            replace_result_ptr: WasmPtr<WasmReplaceResult>,
        ) -> Box<dyn Future<Output = ()> + Send + 'a> {
            Box::new(regex_replace_impl::<Mode>(
                caller,
                ptr,
                source_ptr,
                source_len,
                target_ptr,
                target_len,
                replace_result_ptr,
            ))
        }

        async fn regex_replace_impl<'a, Mode: ReplaceImpl + 'static>(
            mut caller: Caller<'a, WasmStoreData>,
            ptr: i32,
            source_ptr: WasmPtr<u8>,
            source_len: i32,
            target_ptr: WasmPtr<u8>,
            target_len: i32,
            replace_result_ptr: WasmPtr<WasmReplaceResult>,
        ) -> () {
            let state = caller.data().inner.clone();

            let mut ctx = caller.as_context_mut();

            let result = (|| async move {
                let (memory, alloc, result_bytes_len_i32, result) = {
                    let mut state_lock = state.lock().expect("lock");
                    let state = state_lock.deref_mut();
                    let memory = state.memory.expect("no memory");
                    let allocation = state.allocation.as_ref().expect("no allocation");
                    let RegexEnv { regexps, .. } = state.regex.as_ref().expect("RegexEnv");

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

                        let source = ptr_to_utf8(ctx.as_context(), memory, source_ptr, source_len)?;
                        let source = source.as_str()?;

                        let target = ptr_to_utf8(ctx.as_context(), memory, target_ptr, target_len)?;
                        let target = target.as_str()?;

                        let result = Mode::do_replace(&regex.regex, source, target);

                        // TODO: check that `result` is Borrowed and that it has same address as `source`,
                        //       calculate start/end offsets and return partial
                        if result.as_ref() == source {
                            return Ok::<_, WasmError>((
                                memory,
                                ReplaceResult::<WasmPtrImpl> {
                                    is_same: 1,
                                    s: BytesVecFullTrait::null(),
                                },
                            ));
                        }

                        result.into_owned()
                    };

                    let result_bytes_len_i32 = try_usize_to_i32(result.len()).ok_or_else(|| {
                        WasmError::Unspecified(format!(
                            "regex_replace result too big: {}",
                            result.len()
                        ))
                    })?;

                    let alloc = allocation.alloc;

                    (memory, alloc, result_bytes_len_i32, result)
                };

                let vec_ptr = alloc
                    .call_async(ctx.as_context_mut(), result_bytes_len_i32)
                    .await?;

                {
                    let vec_slice = vec_ptr.slice()?;
                    () = vec_slice
                        .write_slice(memory.data_mut(ctx.as_context_mut()), result.as_bytes())?;
                }

                Ok((
                    memory,
                    ReplaceResult::<WasmPtrImpl> {
                        is_same: 0,
                        s: BytesVecRawParts {
                            ptr: vec_ptr.into(),
                            len: result_bytes_len_i32,
                            capacity: result_bytes_len_i32,
                        },
                    },
                ))
            })()
            .await;

            let Some((memory, result)) = WasmEnv::handle_error(&caller.data().error, result) else {
                return ();
            };

            let result = (|| {
                () = replace_result_ptr.write(
                    memory.data_mut(caller.as_context_mut()),
                    WasmReplaceResult(result),
                )?;

                Ok::<(), WasmError>(())
            })();

            () = WasmEnv::handle_error(&caller.data().error, result).unwrap_or(());
        }

        linker.func_wrap("Regex", "new", regex_new)?;
        linker.func_wrap("Regex", "free", regex_free)?;
        linker.func_wrap("Regex", "captures", regex_captures)?;
        linker.func_wrap6_async("Regex", "replace_one", regex_replace::<ReplaceOneImpl>)?;
        linker.func_wrap6_async("Regex", "replace_all", regex_replace::<ReplaceAllImpl>)?;

        Ok(())
    }
}
