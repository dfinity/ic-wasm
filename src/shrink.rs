use crate::metadata::*;
use crate::utils::*;
use walrus::*;

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

#[cfg(feature = "wasm-opt")]
pub fn shrink_with_wasm_opt(
    m: &mut Module,
    level: &str,
    inline_functions_with_loops: bool,
    always_inline_max_function_size: &Option<u32>,
    keep_name_section: bool,
) -> anyhow::Result<()> {
    use tempfile::NamedTempFile;
    use wasm_opt::OptimizationOptions;
    // recursively optimize embedded modules in Motoko actor classes
    if is_motoko_canister(m) {
        let data = get_motoko_wasm_data_sections(m);
        for (id, mut module) in data.into_iter() {
            shrink_with_wasm_opt(
                &mut module,
                level,
                inline_functions_with_loops,
                always_inline_max_function_size,
                keep_name_section,
            )?;
            let blob = encode_module_as_data_section(module);
            m.data.get_mut(id).value = blob;
        }
    }

    // write module to temp file
    let temp_file = NamedTempFile::new()?;
    m.emit_wasm_file(temp_file.path())?;

    // pull out a copy of the custom sections to preserve
    let metadata_sections: Vec<(Kind, &str, Vec<u8>)> = m
        .customs
        .iter()
        .filter(|(_, section)| section.name().starts_with("icp:"))
        .map(|(_, section)| {
            let data = section.data(&IdsToIndices::default()).to_vec();
            let full_name = section.name();
            match full_name.strip_prefix("public ") {
                Some(name) => (Kind::Public, name, data),
                None => match full_name.strip_prefix("private ") {
                    Some(name) => (Kind::Private, name, data),
                    None => unreachable!(),
                },
            }
        })
        .collect();

    // read in from temp file and optimize
    let mut optimizations = match level {
        "O0" => OptimizationOptions::new_opt_level_0(),
        "O1" => OptimizationOptions::new_opt_level_1(),
        "O2" => OptimizationOptions::new_opt_level_2(),
        "O3" => OptimizationOptions::new_opt_level_3(),
        "O4" => OptimizationOptions::new_opt_level_4(),
        "Os" => OptimizationOptions::new_optimize_for_size(),
        "Oz" => OptimizationOptions::new_optimize_for_size_aggressively(),
        _ => anyhow::bail!("invalid optimization level"),
    };
    optimizations.debug_info(keep_name_section);
    optimizations.allow_functions_with_loops(inline_functions_with_loops);
    if let Some(max_size) = always_inline_max_function_size {
        optimizations.always_inline_max_size(*max_size);
    }
    optimizations.run(temp_file.path(), temp_file.path())?;

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
