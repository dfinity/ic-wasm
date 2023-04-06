use crate::utils::*;
use std::path::PathBuf;
use walrus::*;
use wasm_opt::OptimizationOptions;

pub fn optimize(m: &mut Module, keep_name_section: bool) {
    let temp_file_name = "temp.opt.wasm";
    let temp_path = PathBuf::from(temp_file_name);

    // FIXME extract out the custom section first

    // write to fs
    m.emit_wasm_file(temp_file_name).unwrap();

    // read in from fs and optimize
    OptimizationOptions::new_opt_level_3()
        .run(temp_file_name, temp_file_name)
        .unwrap();

    // FIXME re-insert the custom section before assigning back to m

    // read back in from fs and assign back to m
    *m = parse_wasm_file(temp_path, keep_name_section).unwrap();
}
