#[derive(Copy, Clone)]
pub struct RecordFlags(u8);

const VALUE_PRESENT_FLAG: u8 = 0b1;

impl RecordFlags {
    pub fn new() -> Self {
        RecordFlags(0)
    }
    pub fn from_byte(value: u8) -> Self {
        RecordFlags(value)
    }

    pub fn get_byte(&self) -> u8 {
        self.0
    }
}
