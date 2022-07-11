//! A legacy implementation of WASM module optimization for IC
//!
//! https://crates.io/crates/ic-cdk-optimizer
//!
//! Mainly to be used as a lib in dfx.
//! Will be unified with shrink

use binaryen::{CodegenConfig, Module};
use humansize::{file_size_opts, FileSize};
use wabt::{wasm2wat, wat2wasm};

pub type PassResult = Result<Vec<u8>, Box<dyn std::error::Error>>;

pub fn optimize(content: &[u8]) -> PassResult {
    let original_wasm_size = content.len();
    eprintln!(
        "Original:          {:>8}",
        original_wasm_size
            .file_size(file_size_opts::BINARY)
            .unwrap()
    );

    let mut wasm_back = content;

    // strip sections
    eprintln!("Stripping Unused Data Segments...");
    let wat = wasm2wat(&wasm_back)?;
    let wasm_new = wat2wasm(wat)?;
    wasm_back = compare_wasm(wasm_back, &wasm_new);

    // binaryen
    eprintln!("Executing a binaryen optimization...");
    let mut module = Module::read(wasm_back)
        .map_err(|_| String::from("Could not load module for binaryen..."))?;
    module.optimize(&CodegenConfig {
        debug_info: false,
        optimization_level: 2,
        shrink_level: 2,
    });
    let wasm_new = module.write();
    wasm_back = compare_wasm(wasm_back, &wasm_new);

    eprintln!(
        "\nFinal Size: {} ({:3.1}% smaller)",
        wasm_back.len().file_size(file_size_opts::BINARY).unwrap(),
        (1.0 - ((wasm_back.len() as f64) / (original_wasm_size as f64))) * 100.0
    );

    Ok(wasm_back.to_vec())
}

fn compare_wasm<'a>(wasm_back: &'a [u8], wasm_new: &'a [u8]) -> &'a [u8] {
    if wasm_new.len() < wasm_back.len() {
        eprintln!(
            "    Size:          {:>8} ({:3.1}% smaller)",
            wasm_back.len().file_size(file_size_opts::BINARY).unwrap(),
            (1.0 - ((wasm_new.len() as f64) / (wasm_back.len() as f64))) * 100.0
        );
        wasm_new
    } else {
        eprintln!("Pass did not result in smaller WASM... Skipping.");
        wasm_back
    }
}
