use crate::wasm::human_readable::HumanReadableFunctions;
use crate::wasm::{WasmError, WasmModuleInstance};
use crate::Collection;
use std::ops::Deref;
use std::rc::Rc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum HumanReadableError {
    #[error("MultipleWasmModulesAreNotSupportedYet")]
    MultipleWasmModulesAreNotSupportedYet,
    #[error("Collection {0} has no human readable converters")]
    CollectionHasNoHumanReadableConverters(Rc<str>),
    #[error(transparent)]
    Wasm(#[from] WasmError),
}

pub async fn get_collection_human_readable<'a>(
    instance: &'a WasmModuleInstance,
    wasm_module_name: &str,
    collection: &Collection,
) -> Result<HumanReadableFunctions<'a>, HumanReadableError> {
    let Some(hr) = &collection.human_readable else {
        return Err(HumanReadableError::CollectionHasNoHumanReadableConverters(
            collection.name.clone(),
        ));
    };

    if hr.wasm.deref() != wasm_module_name {
        return Err(HumanReadableError::MultipleWasmModulesAreNotSupportedYet);
    }

    let hr = instance
        .human_readable_functions(
            hr.key_to_bytes.as_str(),
            hr.bytes_to_key.as_str(),
            hr.value_to_bytes.as_str(),
            hr.bytes_to_value.as_str(),
        )
        .await?;

    Ok(hr)
}
