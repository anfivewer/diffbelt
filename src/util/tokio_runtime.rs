use tokio::runtime::Runtime;

pub fn create_main_tokio_runtime() -> Result<Runtime, std::io::Error> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    Ok(runtime)
}

pub fn create_single_thread_tokio_runtime() -> Result<Runtime, std::io::Error> {
    tokio::runtime::Builder::new_current_thread().build()
}
