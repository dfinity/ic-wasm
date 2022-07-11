use std::collections::HashMap;
use walrus::*;

#[derive(Clone, Copy, PartialEq)]
pub enum InjectionKind {
    Static,
    Dynamic,
    Dynamic64,
}

pub struct FunctionCost(HashMap<FunctionId, (i64, InjectionKind)>);
impl FunctionCost {
    pub fn new(m: &Module) -> Self {
        let mut res = HashMap::new();
        for (method, func) in m.imports.iter().filter_map(|i| {
            if let ImportKind::Function(func) = i.kind {
                if i.module == "ic0" {
                    Some((i.name.as_str(), func))
                } else {
                    None
                }
            } else {
                None
            }
        }) {
            use InjectionKind::*;
            // System API cost taken from https://github.com/dfinity/ic/blob/master/rs/embedders/src/wasmtime_embedder/system_api_complexity.rs
            let cost = match method {
                "msg_arg_data_copy" => (21, Dynamic),
                "msg_method_name_copy" => (21, Dynamic),
                "msg_reply_data_append" => (21, Dynamic),
                "msg_reject" => (21, Dynamic),
                "msg_reject_msg_copy" => (21, Dynamic),
                "debug_print" => (101, Dynamic),
                "trap" => (21, Dynamic),
                "call_new" => (1, Static),
                "call_data_append" => (21, Dynamic),
                "call_perform" => (1, Static),
                "stable_read" => (21, Dynamic),
                "stable_write" => (21, Dynamic),
                "stable64_read" => (21, Dynamic64),
                "stable64_write" => (21, Dynamic64),
                "performance_counter" => (201, Static),
                _ => (1, Static),
            };
            res.insert(func, cost);
        }
        Self(res)
    }
    pub fn get_cost(&self, id: FunctionId) -> (i64, InjectionKind) {
        *self.0.get(&id).unwrap_or(&(1, InjectionKind::Static))
    }
}

pub fn get_ic_func_id(m: &mut Module, method: &str) -> FunctionId {
    match m.imports.find("ic0", method) {
        Some(id) => match m.imports.get(id).kind {
            ImportKind::Function(func_id) => func_id,
            _ => unreachable!(),
        },
        None => {
            let ty = match method {
                "stable_write" => m
                    .types
                    .add(&[ValType::I32, ValType::I32, ValType::I32], &[]),
                "stable64_write" => m
                    .types
                    .add(&[ValType::I64, ValType::I64, ValType::I64], &[]),
                "stable_read" => m
                    .types
                    .add(&[ValType::I32, ValType::I32, ValType::I32], &[]),
                "stable64_read" => m
                    .types
                    .add(&[ValType::I64, ValType::I64, ValType::I64], &[]),
                "stable_grow" => m.types.add(&[ValType::I32], &[ValType::I32]),
                "stable64_grow" => m.types.add(&[ValType::I64], &[ValType::I64]),
                "stable_size" => m.types.add(&[], &[ValType::I32]),
                "stable64_size" => m.types.add(&[], &[ValType::I64]),
                "call_cycles_add" => m.types.add(&[ValType::I64], &[]),
                "call_cycles_add128" => m.types.add(&[ValType::I64, ValType::I64], &[]),
                "call_new" => m.types.add(
                    &[
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                        ValType::I32,
                    ],
                    &[],
                ),
                "debug_print" => m.types.add(&[ValType::I32, ValType::I32], &[]),
                "trap" => m.types.add(&[ValType::I32, ValType::I32], &[]),
                "msg_reply_data_append" => m.types.add(&[ValType::I32, ValType::I32], &[]),
                "msg_reply" => m.types.add(&[], &[]),
                _ => unreachable!(),
            };
            m.add_import_func("ic0", method, ty).0
        }
    }
}

pub fn get_memory_id(m: &Module) -> MemoryId {
    m.memories
        .iter()
        .next()
        .expect("only single memory is supported")
        .id()
}

pub fn get_export_func_id(m: &Module, method: &str) -> Option<FunctionId> {
    let e = m.exports.iter().find(|e| e.name == method)?;
    if let ExportItem::Function(id) = e.item {
        Some(id)
    } else {
        None
    }
}

pub fn get_builder(m: &mut Module, id: FunctionId) -> InstrSeqBuilder<'_> {
    if let FunctionKind::Local(func) = &mut m.funcs.get_mut(id).kind {
        let id = func.entry_block();
        func.builder_mut().instr_seq(id)
    } else {
        unreachable!()
    }
}

pub fn inject_top(builder: &mut InstrSeqBuilder<'_>, instrs: Vec<ir::Instr>) {
    for instr in instrs.into_iter().rev() {
        builder.instr_at(0, instr);
    }
}

pub fn get_func_name(m: &Module, id: FunctionId) -> String {
    m.funcs
        .get(id)
        .name
        .as_ref()
        .unwrap_or(&format!("func_{}", id.index()))
        .to_string()
}

pub fn is_motoko_canister(m: &Module) -> bool {
    m.customs.iter().any(|(_, s)| {
        s.name() == "icp:private motoko:compiler" || s.name() == "icp:public motoko:compiler"
    }) || m
        .exports
        .iter()
        .any(|e| e.name == "canister_update __motoko_async_helper")
}

pub fn is_motoko_wasm_data_section(blob: &[u8]) -> Option<&[u8]> {
    let len = blob.len() as u32;
    if len > 100
        && blob[0..4] == [0x11, 0x00, 0x00, 0x00]  // tag for blob
        && blob[8..12] == [0x00, 0x61, 0x73, 0x6d]
    // Wasm magic number
    {
        let decoded_len = u32::from_le_bytes(blob[4..8].try_into().unwrap());
        if decoded_len + 8 == len {
            return Some(&blob[8..]);
        }
    }
    None
}

pub fn get_motoko_wasm_data_sections(m: &Module) -> Vec<(DataId, Module)> {
    m.data
        .iter()
        .filter_map(|d| {
            let blob = is_motoko_wasm_data_section(&d.value)?;
            let mut config = ModuleConfig::new();
            config.generate_name_section(false);
            config.generate_producers_section(false);
            let m = config.parse(blob).ok()?;
            Some((d.id(), m))
        })
        .collect()
}

pub fn encode_module_as_data_section(mut m: Module) -> Vec<u8> {
    let blob = m.emit_wasm();
    let blob_len = blob.len();
    let mut res = Vec::with_capacity(blob_len + 8);
    res.extend_from_slice(&[0x11, 0x00, 0x00, 0x00]);
    let encoded_len = (blob_len as u32).to_le_bytes();
    res.extend_from_slice(&encoded_len);
    res.extend_from_slice(&blob);
    res
}
