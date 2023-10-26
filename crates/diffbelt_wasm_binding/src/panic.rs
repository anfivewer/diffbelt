#[cfg(all(feature = "panic", target_arch = "wasm32"))]
use crate::debug_print::debug_print_string;
#[cfg(all(feature = "panic", target_arch = "wasm32"))]
use alloc::string::ToString;

#[cfg(all(feature = "panic", target_arch = "wasm32"))]
#[panic_handler]
fn panic(panic: &core::panic::PanicInfo<'_>) -> ! {
    debug_print_string(panic.to_string());

    core::arch::wasm32::unreachable();
}
