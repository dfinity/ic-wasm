use crate::{utils::*, Error};
use walrus::*;

pub fn shrink(wasm: &[u8]) -> Result<Vec<u8>, Error> {
    let mut config = walrus::ModuleConfig::new();
    config.generate_name_section(true);
    config.generate_producers_section(false);
    let mut m = config
        .parse(wasm)
        .map_err(|e| Error::WASM(format!("Could not parse the data as WASM module. {}", e)))?;
    shrink_(&mut m);
    Ok(m.emit_wasm())
}

fn shrink_(m: &mut Module) {
    if is_motoko_canister(m) {
        let ids = get_motoko_wasm_data_sections(m);
        for (id, mut module) in ids.into_iter() {
            shrink_(&mut module);
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
