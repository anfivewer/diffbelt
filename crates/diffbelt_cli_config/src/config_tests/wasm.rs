use crate::config_tests::options::RunTestsContext;
use crate::wasm::WasmError;
use crate::CliConfig;
use std::path::PathBuf;

impl CliConfig {
    pub async fn init_run_tests_context(
        &self,
        context: &mut RunTestsContext,
    ) -> Result<(), WasmError> {
        for wasm_def in self.wasm.values() {
            let name = wasm_def.name.as_ref();
            let path = wasm_def.wasm_path.value.as_str();

            context
                .wasm_engine
                .register_module(name, PathBuf::from(path))
                .await?;
        }

        Ok(())
    }
}
