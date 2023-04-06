use crate::metadata::*;
use crate::utils::*;
use std::path::PathBuf;
use walrus::*;
use wasm_opt::OptimizationOptions;

pub fn optimize(m: &mut Module, keep_name_section: bool) {
    let temp_file_name = "temp.opt.wasm";
    let temp_path = PathBuf::from(temp_file_name);

    // pull out the custom sections to preserve
    let mut metadata_sections = Vec::new();
    list_metadata(m).iter().for_each(|full_name| {
        match full_name.strip_prefix("icp:public ") {
            Some(name) => metadata_sections.push(("public", name, get_metadata(m, name).unwrap())),
            None => match full_name.strip_prefix("icp:private ") {
                Some(name) => {
                    metadata_sections.push(("private", name, get_metadata(m, name).unwrap()))
                }
                None => unreachable!(),
            },
        };
    });

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
