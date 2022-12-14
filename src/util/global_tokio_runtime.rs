use std::sync::Arc;
use tokio::runtime::Runtime;

static mut TOKIO_RUNTIME: Option<Arc<Runtime>> = None;

pub fn create_global_tokio_runtime() -> Result<Arc<Runtime>, std::io::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let runtime = Arc::new(runtime);

    unsafe {
        TOKIO_RUNTIME = Some(runtime.clone());
    }

    Ok(runtime)
}

pub fn get_global_tokio_runtime_or_panic() -> Arc<Runtime> {
    unsafe {
        match &TOKIO_RUNTIME {
            Some(runtime) => runtime.clone(),
            None => {
                panic!("no tokio runtime");
            }
        }
    }
}
