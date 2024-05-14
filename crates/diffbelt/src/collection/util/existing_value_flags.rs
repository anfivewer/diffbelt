#[derive(Copy, Clone)]
pub struct ExistingValueFlags(u8);

impl ExistingValueFlags {
    pub fn new() -> Self {
        ExistingValueFlags(0)
    }

    pub fn get_byte(&self) -> u8 {
        self.0
    }
}
