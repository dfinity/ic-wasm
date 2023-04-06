use crate::metadata::*;
use crate::utils::*;
use std::path::PathBuf;
use walrus::*;
use wasm_opt::OptimizationOptions;

pub fn optimize(m: &mut Module, keep_name_section: bool, level: &str) {
    let temp_file_name = "temp.opt.wasm";
    let temp_path = PathBuf::from(temp_file_name);

    // pull out a copy of the custom sections to preserve
    let m_copy = parse_wasm(&m.emit_wasm(), keep_name_section).unwrap();
    let mut metadata_sections = Vec::new();
    list_metadata(&m_copy).iter().for_each(|full_name| {
        match full_name.strip_prefix("icp:public ") {
            Some(name) => {
                metadata_sections.push(("public", name, get_metadata(&m_copy, name).unwrap()))
            }
            None => match full_name.strip_prefix("icp:private ") {
                Some(name) => {
                    metadata_sections.push(("private", name, get_metadata(&m_copy, name).unwrap()))
                }
                None => unreachable!(),
            },
        };
    });

    // write to fs
    m.emit_wasm_file(temp_file_name).unwrap();

    // read in from fs and optimize
    match level {
        "O0" => OptimizationOptions::new_opt_level_0(),
        "O1" => OptimizationOptions::new_opt_level_1(),
        "O2" => OptimizationOptions::new_opt_level_2(),
        "O3" => OptimizationOptions::new_opt_level_3(),
        "O4" => OptimizationOptions::new_opt_level_4(),
        "Os" => OptimizationOptions::new_optimize_for_size(),
        "Oz" => OptimizationOptions::new_optimize_for_size_aggressively(),
        _ => unreachable!(),
    }
    .run(temp_file_name, temp_file_name)
    .unwrap();

    // read optimized wasm back in from fs
    *m = parse_wasm_file(temp_path, keep_name_section).unwrap();

    // re-insert the custom sections
    metadata_sections
        .iter()
        .for_each(|(visibility, name, data)| {
            let visibility = match *visibility {
                "public" => Kind::Public,
                "private" => Kind::Private,
                _ => unreachable!(),
            };
            add_metadata(m, visibility, name, data.to_vec());
        });
}
