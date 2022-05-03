use crate::utils::{get_motoko_wasm_data_sections, is_motoko_canister};
use walrus::*;

pub fn shrink(m: &mut Module) {
    if is_motoko_canister(m) {
        let ids = get_motoko_wasm_data_sections(m);
        for (id, mut module) in ids.into_iter() {
            shrink(&mut module);
            let blob = module.emit_wasm();
            let blob_len = blob.len() as u32;
            let original_len = m.data.get(id).value.len() as u32;
            if blob_len + 8 < original_len {
                let data = &mut m.data.get_mut(id).value;
                data.truncate(4);
                let encoded_len = blob_len.to_le_bytes();
                data.extend_from_slice(&encoded_len);
                data.extend_from_slice(&blob);
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
