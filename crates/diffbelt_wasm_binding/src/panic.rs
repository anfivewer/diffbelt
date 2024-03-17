#[macro_export]
macro_rules! define_panic_handler {
    () => {
        #[panic_handler]
        fn panic(panic: &::core::panic::PanicInfo<'_>) -> ! {
            ::diffbelt_wasm_binding::debug_print_string(::alloc::string::ToString::to_string(
                panic,
            ));

            ::core::arch::wasm32::unreachable();
        }
    };
}
