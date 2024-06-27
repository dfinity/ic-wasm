use crate::metadata::*;
use crate::utils::*;
use clap::ValueEnum;
use walrus::*;

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OptLevel {
    #[clap(name = "O0")]
    O0,
    #[clap(name = "O1")]
    O1,
    #[clap(name = "O2")]
    O2,
    #[clap(name = "O3")]
    O3,
    #[clap(name = "O4")]
    O4,
    #[clap(name = "Os")]
    Os,
    #[clap(name = "Oz")]
    Oz,
}

pub fn optimize(
    m: &mut Module,
    level: &OptLevel,
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
            let old_size = module.emit_wasm().len();
            optimize(
                &mut module,
                level,
                inline_functions_with_loops,
                always_inline_max_function_size,
                keep_name_section,
            )?;
            let new_size = module.emit_wasm().len();
            // Guard against embedded actor class overriding the parent module
            if new_size <= old_size {
                let blob = encode_module_as_data_section(module);
                m.data.get_mut(id).value = blob;
            } else {
                eprintln!("Warning: embedded actor class module was not optimized because the optimized module is larger than the original module");
            }
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
        OptLevel::O0 => OptimizationOptions::new_opt_level_0(),
        OptLevel::O1 => OptimizationOptions::new_opt_level_1(),
        OptLevel::O2 => OptimizationOptions::new_opt_level_2(),
        OptLevel::O3 => OptimizationOptions::new_opt_level_3(),
        OptLevel::O4 => OptimizationOptions::new_opt_level_4(),
        OptLevel::Os => OptimizationOptions::new_optimize_for_size(),
        OptLevel::Oz => OptimizationOptions::new_optimize_for_size_aggressively(),
    };
    optimizations.debug_info(keep_name_section);
    optimizations.allow_functions_with_loops(inline_functions_with_loops);
    if let Some(max_size) = always_inline_max_function_size {
        optimizations.always_inline_max_size(*max_size);
    }
    // The feature set should be align with IC `wasmtime` validation config:
    // https://github.com/dfinity/ic/blob/6a6470d705a0f36fb94743b12892280409f85688/rs/embedders/src/wasm_utils/validation.rs#L1385
    optimizations.enable_feature(wasm_opt::Feature::MutableGlobals);
    optimizations.enable_feature(wasm_opt::Feature::TruncSat);
    optimizations.enable_feature(wasm_opt::Feature::Simd);
    optimizations.enable_feature(wasm_opt::Feature::BulkMemory);
    optimizations.enable_feature(wasm_opt::Feature::SignExt);
    optimizations.enable_feature(wasm_opt::Feature::ReferenceTypes);
    optimizations.enable_feature(wasm_opt::Feature::Memory64);
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
