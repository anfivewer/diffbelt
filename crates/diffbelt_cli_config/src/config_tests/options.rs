pub struct RunTestsOptions {
    pub with_unit: bool,
    pub with_integration: bool,
}

impl Default for RunTestsOptions {
    fn default() -> Self {
        Self {
            with_unit: true,
            with_integration: true,
        }
    }
}