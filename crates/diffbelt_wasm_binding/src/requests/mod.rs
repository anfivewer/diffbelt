#[repr(C)]
pub struct RequestId(u32);

#[link(wasm_import_module = "Diffbelt")]
extern "C" {
    fn request() -> RequestId;
}
