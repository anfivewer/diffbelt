/// SAFETY: The runtime environment must be single-threaded WASM.
#[global_allocator]
#[cfg(all(target_family = "wasm"))]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };
