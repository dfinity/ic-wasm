pub mod info;
pub mod instrumentation;
pub mod limit_resource;
pub mod metadata;
pub mod shrink;
pub mod utils;

use thiserror;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("")]
    IO(#[from] std::io::Error),

    #[error("{0}")]
    WASM(String)
}