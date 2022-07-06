//! A legacy implementation of WASM module optimization for IC
//!
//! https://crates.io/crates/ic-cdk-optimizer
//!
//! Mainly to be used as a lib in dfx.
//! Will be unified with shrink

use binaryen::{CodegenConfig, Module};
use wabt::{wasm2wat, wat2wasm};

pub type PassResult = Result<Vec<u8>, Box<dyn std::error::Error>>;

pub fn optimize(content: &[u8]) -> PassResult {
    let mut wasm_back = content;

    // strip sections
    let wat = wasm2wat(&wasm_back)?;
    let wasm_new = wat2wasm(wat)?;
    if wasm_new.len() < wasm_back.len() {
        wasm_back = &wasm_new;
    }

    // binaryen
    let mut module = Module::read(wasm_back)
        .map_err(|_| String::from("Could not load module for binaryen..."))?;
    module.optimize(&CodegenConfig {
        debug_info: false,
        optimization_level: 2,
        shrink_level: 2,
    });
    let wasm_new = module.write();
    if wasm_new.len() < wasm_back.len() {
        wasm_back = &wasm_new;
    }

    Ok(wasm_back.to_vec())
}
