use crate::metadata::*;
use crate::utils::*;
use tempfile::NamedTempFile;
use walrus::*;
use wasm_opt::OptimizationOptions;

pub fn shrink(m: &mut Module) {
    if is_motoko_canister(m) {
        let ids = get_motoko_wasm_data_sections(m);
        for (id, mut module) in ids.into_iter() {
            shrink(&mut module);
            let blob = encode_module_as_data_section(module);
            let original_len = m.data.get(id).value.len();
            if blob.len() < original_len {
                m.data.get_mut(id).value = blob;
            }
        }
    }
    let to_remove: Vec<_> = m
        .customs
        .iter()
        .filter(|(_, section)| !section.name().starts_with("icp:"))
        .map(|(id, _)| id)
        .collect();
    for s in to_remove {
        m.customs.delete(s);
    }
    passes::gc::run(m);
}

pub fn optimize(m: &mut Module, keep_name_section: bool, level: &str) -> anyhow::Result<()> {
    // recursively optimize embedded modules in Motoko actor classes
    if is_motoko_canister(m) {
        let data = get_motoko_wasm_data_sections(m);
        for (id, mut module) in data.into_iter() {
            optimize(&mut module, keep_name_section, level)?;
            let blob = encode_module_as_data_section(module);
            m.data.get_mut(id).value = blob;
        }
    }

    // write module to temp file
    let temp_file = NamedTempFile::new()?;
    m.emit_wasm_file(temp_file.path())?;

    // pull out a copy of the custom sections to preserve
    let mut metadata_sections: Vec<(Kind, &str, Vec<u8>)> = Vec::new();
    list_metadata(m).iter().for_each(|full_name| {
        match full_name.strip_prefix("icp:public ") {
            Some(name) => metadata_sections.push((
                Kind::Public,
                name,
                get_metadata(m, name).unwrap().to_vec(),
            )),
            None => match full_name.strip_prefix("icp:private ") {
                Some(name) => metadata_sections.push((
                    Kind::Private,
                    name,
                    get_metadata(m, name).unwrap().to_vec(),
                )),
                None => unreachable!(),
            },
        };
    });

    // read in from temp file and optimize
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
    .run(temp_file.path(), temp_file.path())?;

    // read optimized wasm back in from temp file
    let mut m_opt = parse_wasm_file(temp_file.path().to_path_buf(), keep_name_section)?;

    // re-insert the custom sections
    metadata_sections
        .into_iter()
        .for_each(|(visibility, name, data)| {
            add_metadata(&mut m_opt, visibility, name, data);
        });

    *m = m_opt;
    Ok(())
}
