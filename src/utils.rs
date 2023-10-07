use crate::Error;
use std::collections::HashMap;
use walrus::*;

fn wasm_parser_config(keep_name_section: bool) -> ModuleConfig {
    let mut config = walrus::ModuleConfig::new();
    config.generate_name_section(keep_name_section);
    config.generate_producers_section(false);
    config
}

pub fn parse_wasm(wasm: &[u8], keep_name_section: bool) -> Result<Module, Error> {
    let config = wasm_parser_config(keep_name_section);
    config
        .parse(wasm)
        .map_err(|e| Error::WasmParse(e.to_string()))
}

pub fn parse_wasm_file(file: std::path::PathBuf, keep_name_section: bool) -> Result<Module, Error> {
    let config = wasm_parser_config(keep_name_section);
    config
        .parse_file(file)
        .map_err(|e| Error::WasmParse(e.to_string()))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum InjectionKind {
    Static,
    Dynamic,
    Dynamic64,
}

pub(crate) struct FunctionCost(HashMap<FunctionId, (i64, InjectionKind)>);
impl FunctionCost {
    pub fn new(m: &Module, use_new_metering: bool) -> Self {
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
            let cost = if use_new_metering {
                match method {
                    "accept_message" => (500, Static),
                    "call_cycles_add" | "call_cycles_add128" => (500, Static),
                    "call_data_append" => (500, Dynamic),
                    "call_new" => (1500, Static),
                    "call_on_cleanup" => (500, Static),
                    "call_perform" => (5000, Static),
                    "canister_cycle_balance" | "canister_cycle_balance128" => (500, Static),
                    "canister_self_copy" => (500, Dynamic),
                    "canister_self_size" => (500, Static),
                    "canister_status" | "canister_version" => (500, Static),
                    "certified_data_set" => (500, Dynamic),
                    "data_certificate_copy" => (500, Dynamic),
                    "data_certificate_present" | "data_certificate_size" => (500, Static),
                    "debug_print" => (100, Dynamic),
                    "global_timer_set" => (500, Static),
                    "is_controller" => (1000, Dynamic),
                    "msg_arg_data_copy" => (500, Dynamic),
                    "msg_arg_data_size" => (500, Static),
                    "msg_caller_copy" => (500, Dynamic),
                    "msg_caller_size" => (500, Static),
                    "msg_cycles_accept" | "msg_cycles_accept128" => (500, Static),
                    "msg_cycles_available" | "msg_cycles_available128" => (500, Static),
                    "msg_cycles_refunded" | "msg_cycles_refunded128" => (500, Static),
                    "msg_method_name_copy" => (500, Dynamic),
                    "msg_method_name_size" => (500, Static),
                    "msg_reject_code" | "msg_reject_msg_size" => (500, Static),
                    "msg_reject_msg_copy" => (500, Dynamic),
                    "msg_reject" => (500, Dynamic),
                    "msg_reply_data_append" => (500, Dynamic),
                    "msg_reply" => (500, Static),
                    "performance_counter" => (200, Static),
                    "stable_grow" | "stable64_grow" => (100, Static),
                    "stable_size" | "stable64_size" => (20, Static),
                    "stable_read" => (20, Dynamic),
                    "stable_write" => (20, Dynamic),
                    "stable64_read" => (20, Dynamic64),
                    "stable64_write" => (20, Dynamic64),
                    "trap" => (500, Dynamic),
                    "time" => (500, Static),
                    _ => (1, Static),
                }
            } else {
                match method {
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
                }
            };
            res.insert(func, cost);
        }
        Self(res)
    }
    pub fn get_cost(&self, id: FunctionId) -> (i64, InjectionKind) {
        *self.0.get(&id).unwrap_or(&(1, InjectionKind::Static))
    }
}
pub(crate) fn instr_cost(i: &ir::Instr, use_new_metering: bool) -> i64 {
    use ir::*;
    use BinaryOp::*;
    use UnaryOp::*;
    if !use_new_metering {
        return 1;
    }
    match i {
        Instr::Block(..) | Instr::Loop(..) => 0,
        Instr::Const(..) | Instr::Load(..) | Instr::Store(..) => 1,
        Instr::GlobalGet(..) | Instr::GlobalSet(..) => 2,
        Instr::TableGet(..) | Instr::TableSet(..) => 5,
        Instr::TableGrow(..) | Instr::MemoryGrow(..) => 300,
        Instr::MemorySize(..) => 20,
        Instr::TableSize(..) => 100,
        Instr::MemoryFill(..) | Instr::MemoryCopy(..) | Instr::MemoryInit(..) => 100,
        Instr::TableFill(..) | Instr::TableCopy(..) | Instr::TableInit(..) => 100,
        Instr::DataDrop(..) | Instr::ElemDrop(..) => 300,
        Instr::Call(..) => 5,
        Instr::CallIndirect(..) => 10, // missing ReturnCall/Indirect
        Instr::IfElse(..) | Instr::Br(..) | Instr::BrIf(..) | Instr::BrTable(..) => 2,
        Instr::RefIsNull(..) => 5,
        Instr::RefFunc(..) => 130,
        Instr::Unop(Unop { op }) => match op {
            F32Ceil | F32Floor | F32Trunc | F32Nearest | F32Sqrt => 20,
            F64Ceil | F64Floor | F64Trunc | F64Nearest | F64Sqrt => 20,
            F32Abs | F32Neg | F64Abs | F64Neg => 2,
            F32ConvertSI32 | F64ConvertSI64 | F32ConvertSI64 | F64ConvertSI32 => 3,
            F64ConvertUI32 | F32ConvertUI64 | F32ConvertUI32 | F64ConvertUI64 => 16,
            I64TruncSF32 | I64TruncUF32 | I64TruncSF64 | I64TruncUF64 => 20,
            I32TruncSF32 | I32TruncUF32 | I32TruncSF64 | I32TruncUF64 => 20, // missing TruncSat?
            _ => 1,
        },
        Instr::Binop(Binop { op }) => match op {
            I32DivS | I32DivU | I32RemS | I32RemU => 10,
            I64DivS | I64DivU | I64RemS | I64RemU => 10,
            F32Add | F32Sub | F32Mul | F32Div | F32Min | F32Max => 20,
            F64Add | F64Sub | F64Mul | F64Div | F64Min | F64Max => 20,
            F32Copysign | F64Copysign => 2,
            F32Eq | F32Ne | F32Lt | F32Gt | F32Le | F32Ge => 3,
            F64Eq | F64Ne | F64Lt | F64Gt | F64Le | F64Ge => 3,
            _ => 1,
        },
        _ => 1,
    }
}

pub(crate) fn get_ic_func_id(m: &mut Module, method: &str) -> FunctionId {
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

pub(crate) fn get_memory_id(m: &Module) -> MemoryId {
    m.memories
        .iter()
        .next()
        .expect("only single memory is supported")
        .id()
}

pub(crate) fn get_export_func_id(m: &Module, method: &str) -> Option<FunctionId> {
    let e = m.exports.iter().find(|e| e.name == method)?;
    if let ExportItem::Function(id) = e.item {
        Some(id)
    } else {
        None
    }
}
pub(crate) fn get_or_create_export_func<'a>(
    m: &'a mut Module,
    method: &'a str,
) -> InstrSeqBuilder<'a> {
    let id = match get_export_func_id(m, method) {
        Some(id) => id,
        None => {
            let builder = FunctionBuilder::new(&mut m.types, &[], &[]);
            let id = builder.finish(vec![], &mut m.funcs);
            m.exports.add(method, id);
            id
        }
    };
    get_builder(m, id)
}

pub(crate) fn get_builder(m: &mut Module, id: FunctionId) -> InstrSeqBuilder<'_> {
    if let FunctionKind::Local(func) = &mut m.funcs.get_mut(id).kind {
        let id = func.entry_block();
        func.builder_mut().instr_seq(id)
    } else {
        unreachable!()
    }
}

pub(crate) fn inject_top(builder: &mut InstrSeqBuilder<'_>, instrs: Vec<ir::Instr>) {
    for instr in instrs.into_iter().rev() {
        builder.instr_at(0, instr);
    }
}

pub(crate) fn get_func_name(m: &Module, id: FunctionId) -> String {
    m.funcs
        .get(id)
        .name
        .as_ref()
        .unwrap_or(&format!("func_{}", id.index()))
        .to_string()
}

pub(crate) fn is_motoko_canister(m: &Module) -> bool {
    m.customs.iter().any(|(_, s)| {
        s.name() == "icp:private motoko:compiler" || s.name() == "icp:public motoko:compiler"
    }) || m
        .exports
        .iter()
        .any(|e| e.name == "canister_update __motoko_async_helper")
}

pub(crate) fn is_motoko_wasm_data_section(blob: &[u8]) -> Option<&[u8]> {
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

pub(crate) fn get_motoko_wasm_data_sections(m: &Module) -> Vec<(DataId, Module)> {
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

pub(crate) fn encode_module_as_data_section(mut m: Module) -> Vec<u8> {
    let blob = m.emit_wasm();
    let blob_len = blob.len();
    let mut res = Vec::with_capacity(blob_len + 8);
    res.extend_from_slice(&[0x11, 0x00, 0x00, 0x00]);
    let encoded_len = (blob_len as u32).to_le_bytes();
    res.extend_from_slice(&encoded_len);
    res.extend_from_slice(&blob);
    res
}
