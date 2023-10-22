use crate::debug_print::debug_print_string;
use alloc::string::ToString;

#[panic_handler]
fn panic(panic: &core::panic::PanicInfo<'_>) -> ! {
    debug_print_string(panic.to_string());

    core::arch::wasm32::unreachable()
}
