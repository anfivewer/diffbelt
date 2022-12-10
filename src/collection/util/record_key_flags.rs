#[derive(Copy, Clone)]
pub struct RecordKeyFlags(u8);

const VALUE_PRESENT_FLAG: u8 = 0b1;

impl RecordKeyFlags {
    pub fn new() -> Self {
        RecordKeyFlags(0)
    }

    pub fn is_value_present(&self) -> bool {
        self.0 & VALUE_PRESENT_FLAG == VALUE_PRESENT_FLAG
    }
    pub fn set_value_is_present(&mut self, is_present: bool) {
        if is_present {
            self.0 = self.0 | VALUE_PRESENT_FLAG;
        } else {
            self.0 = self.0 & !VALUE_PRESENT_FLAG;
        }
    }

    pub fn get_byte(&self) -> u8 {
        self.0
    }
}
