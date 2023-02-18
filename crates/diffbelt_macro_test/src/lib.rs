use diffbelt_macro::fn_box_pin_async;

#[fn_box_pin_async]
pub async fn test_fn() -> usize {
    42
}
