pub mod info;
pub mod instrumentation;
pub mod limit_resource;
pub mod metadata;
pub mod shrink;
pub mod utils;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed on IO.")]
    IO(#[from] std::io::Error),

    #[error("Could not parse the data as WASM module. {0}")]
    WasmParse(String),

    #[error("{0}")]
    MetadataNotFound(String),
}
