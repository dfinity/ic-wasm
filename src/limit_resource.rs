use candid::Principal;
use std::collections::{HashMap, HashSet};
use walrus::ir::*;
use walrus::*;

pub struct Config {
    pub remove_cycles_add: bool,
    pub limit_stable_memory_page: Option<u32>,
    pub limit_heap_memory_page: Option<u32>,
    pub playground_canister_id: Option<candid::Principal>,
}

struct Replacer(HashMap<FunctionId, FunctionId>);
impl VisitorMut for Replacer {
    fn visit_instr_mut(&mut self, instr: &mut Instr, _: &mut InstrLocId) {
        if let Instr::Call(walrus::ir::Call { func }) = instr {
            if let Some(new_id) = self.0.get(func) {
                *instr = Call { func: *new_id }.into();
            }
        }
    }
}
impl Replacer {
    fn new() -> Self {
        Self(HashMap::new())
    }
    fn add(&mut self, old: FunctionId, new: FunctionId) {
        self.0.insert(old, new);
    }
}

pub fn limit_resource(m: &mut Module, config: &Config) {
    let wasm64 = match m.memories.len() {
        0 => false, // Wasm module declares no memory is treated as wasm32
        1 => m.memories.get(m.get_memory_id().unwrap()).memory64,
        _ => panic!("The Canister Wasm module should have at most one memory"),
    };

    if let Some(limit) = config.limit_heap_memory_page {
        limit_heap_memory(m, limit);
    }

    let mut replacer = Replacer::new();

    if config.remove_cycles_add {
        make_cycles_add(m, &mut replacer, wasm64);
        make_cycles_add128(m, &mut replacer);
        make_cycles_burn128(m, &mut replacer, wasm64);
    }

    if let Some(limit) = config.limit_stable_memory_page {
        make_stable_grow(m, &mut replacer, wasm64, limit as i32);
        make_stable64_grow(m, &mut replacer, limit as i64);
    }

    if let Some(redirect_id) = config.playground_canister_id {
        make_redirect_call_new(m, &mut replacer, wasm64, redirect_id);
    }

    let new_ids = replacer.0.values().cloned().collect::<HashSet<_>>();
    m.funcs.iter_local_mut().for_each(|(id, func)| {
        if new_ids.contains(&id) {
            return;
        }
        dfs_pre_order_mut(&mut replacer, func, func.entry_block());
    });
}

fn limit_heap_memory(m: &mut Module, limit: u32) {
    if let Ok(memory_id) = m.get_memory_id() {
        let memory = m.memories.get_mut(memory_id);
        let limit = limit as u64;
        if memory.initial > limit {
            // If memory.initial is greater than the provided limit, it is
            // possible there is an active data segment with an offset in the
            // range [limit, memory.initial].
            //
            // In that case, we don't restrict the heap memory limit as it could
            // have undefined behaviour.

            if m.data
                .iter()
                .filter_map(|data| {
                    match data.kind {
                        DataKind::Passive => None,
                        DataKind::Active {
                            memory: data_memory_id,
                            offset,
                        } => {
                            if data_memory_id == memory_id {
                                match offset {
                                    ConstExpr::Value(Value::I32(offset)) => Some(offset as u64),
                                    ConstExpr::Value(Value::I64(offset)) => Some(offset as u64),
                                    _ => {
                                        // It wouldn't pass IC wasm validation
                                        None
                                    }
                                }
                            } else {
                                None
                            }
                        }
                    }
                })
                .all(|offset| offset < limit * 65536)
            {
                memory.initial = limit;
            } else {
                panic!("Unable to restrict Wasm heap memory to {limit} pages");
            }
        }
        memory.maximum = Some(limit);
    }
}

fn make_cycles_add(m: &mut Module, replacer: &mut Replacer, wasm64: bool) {
    if let Some(old_cycles_add) = get_ic_func_id(m, "call_cycles_add") {
        if wasm64 {
            panic!("Wasm64 module should not call `call_cycles_add`");
        }
        let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64], &[]);
        let amount = m.locals.add(ValType::I64);
        builder.func_body().local_get(amount).drop();
        let new_cycles_add = builder.finish(vec![amount], &mut m.funcs);
        replacer.add(old_cycles_add, new_cycles_add);
    }
}

fn make_cycles_add128(m: &mut Module, replacer: &mut Replacer) {
    if let Some(old_cycles_add128) = get_ic_func_id(m, "call_cycles_add128") {
        let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64, ValType::I64], &[]);
        let high = m.locals.add(ValType::I64);
        let low = m.locals.add(ValType::I64);
        builder
            .func_body()
            .local_get(high)
            .local_get(low)
            .drop()
            .drop();
        let new_cycles_add128 = builder.finish(vec![high, low], &mut m.funcs);
        replacer.add(old_cycles_add128, new_cycles_add128);
    }
}

fn make_cycles_burn128(m: &mut Module, replacer: &mut Replacer, wasm64: bool) {
    if let Some(older_cycles_burn128) = get_ic_func_id(m, "call_cycles_burn128") {
        let dst_type = match wasm64 {
            true => ValType::I64,
            false => ValType::I32,
        };
        let mut builder =
            FunctionBuilder::new(&mut m.types, &[ValType::I64, ValType::I64, dst_type], &[]);
        let high = m.locals.add(ValType::I64);
        let low = m.locals.add(ValType::I64);
        let dst = m.locals.add(dst_type);
        builder
            .func_body()
            .local_get(high)
            .local_get(low)
            .local_get(dst)
            .drop()
            .drop()
            .drop();
        let new_cycles_burn128 = builder.finish(vec![high, low, dst], &mut m.funcs);
        replacer.add(older_cycles_burn128, new_cycles_burn128);
    }
}

fn make_stable_grow(m: &mut Module, replacer: &mut Replacer, wasm64: bool, limit: i32) {
    if let Some(old_stable_grow) = get_ic_func_id(m, "stable_grow") {
        if wasm64 {
            panic!("Wasm64 module should not call `stable_grow`");
        }
        // stable_size is added to import if it wasn't imported
        let stable_size = get_ic_func_id(m, "stable_size").unwrap();
        let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I32], &[ValType::I32]);
        let requested = m.locals.add(ValType::I32);
        builder
            .func_body()
            .call(stable_size)
            .local_get(requested)
            .binop(BinaryOp::I32Add)
            .i32_const(limit)
            .binop(BinaryOp::I32GtU)
            .if_else(
                Some(ValType::I32),
                |then| {
                    then.i32_const(-1);
                },
                |else_| {
                    else_.local_get(requested).call(old_stable_grow);
                },
            );
        let new_stable_grow = builder.finish(vec![requested], &mut m.funcs);
        replacer.add(old_stable_grow, new_stable_grow);
    }
}

fn make_stable64_grow(m: &mut Module, replacer: &mut Replacer, limit: i64) {
    if let Some(old_stable64_grow) = get_ic_func_id(m, "stable64_grow") {
        // stable64_size is added to import if it wasn't imported
        let stable64_size = get_ic_func_id(m, "stable64_size").unwrap();
        let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64], &[ValType::I64]);
        let requested = m.locals.add(ValType::I64);
        builder
            .func_body()
            .call(stable64_size)
            .local_get(requested)
            .binop(BinaryOp::I64Add)
            .i64_const(limit)
            .binop(BinaryOp::I64GtU)
            .if_else(
                Some(ValType::I64),
                |then| {
                    then.i64_const(-1);
                },
                |else_| {
                    else_.local_get(requested).call(old_stable64_grow);
                },
            );
        let new_stable64_grow = builder.finish(vec![requested], &mut m.funcs);
        replacer.add(old_stable64_grow, new_stable64_grow);
    }
}

#[allow(clippy::too_many_arguments)]
fn check_list(
    memory: MemoryId,
    checks: &mut InstrSeqBuilder,
    no_redirect: LocalId,
    size: LocalId,
    src: LocalId,
    is_rename: Option<LocalId>,
    list: &Vec<&[u8]>,
    wasm64: bool,
) {
    let checks_id = checks.id();
    for bytes in list {
        checks.block(None, |list_check| {
            let list_check_id = list_check.id();
            // Check the length
            list_check.local_get(size);
            match wasm64 {
                true => {
                    list_check
                        .i64_const(bytes.len() as i64)
                        .binop(BinaryOp::I64Ne);
                }
                false => {
                    list_check
                        .i32_const(bytes.len() as i32)
                        .binop(BinaryOp::I32Ne);
                }
            }
            list_check.br_if(list_check_id);
            // Load bytes at src onto the stack
            for i in 0..bytes.len() {
                list_check.local_get(src).load(
                    memory,
                    match wasm64 {
                        true => LoadKind::I64_8 {
                            kind: ExtendedLoad::ZeroExtend,
                        },
                        false => LoadKind::I32_8 {
                            kind: ExtendedLoad::ZeroExtend,
                        },
                    },
                    MemArg {
                        offset: i as u32,
                        align: 1,
                    },
                );
            }
            for byte in bytes.iter().rev() {
                match wasm64 {
                    true => {
                        list_check.i64_const(*byte as i64).binop(BinaryOp::I64Ne);
                    }
                    false => {
                        list_check.i32_const(*byte as i32).binop(BinaryOp::I32Ne);
                    }
                }
                list_check.br_if(list_check_id);
            }
            // names were equal, so skip all remaining checks and redirect
            if let Some(is_rename) = is_rename {
                if bytes == b"http_request" {
                    list_check.i32_const(1).local_set(is_rename);
                } else {
                    list_check.i32_const(0).local_set(is_rename);
                }
            }
            list_check.i32_const(0).local_set(no_redirect).br(checks_id);
        });
    }
    // None matched
    checks.i32_const(1).local_set(no_redirect);
}

fn make_redirect_call_new(
    m: &mut Module,
    replacer: &mut Replacer,
    wasm64: bool,
    redirect_id: Principal,
) {
    if let Some(old_call_new) = get_ic_func_id(m, "call_new") {
        let pointer_type = match wasm64 {
            true => ValType::I64,
            false => ValType::I32,
        };
        let redirect_id = redirect_id.as_slice();
        // Specify the same args as `call_new` so that WASM will correctly check mismatching args
        let callee_src = m.locals.add(pointer_type);
        let callee_size = m.locals.add(pointer_type);
        let name_src = m.locals.add(pointer_type);
        let name_size = m.locals.add(pointer_type);
        let arg5 = m.locals.add(pointer_type);
        let arg6 = m.locals.add(pointer_type);
        let arg7 = m.locals.add(pointer_type);
        let arg8 = m.locals.add(pointer_type);

        let memory = m
            .get_memory_id()
            .expect("Canister Wasm module should have only one memory");

        // Scratch variables
        let no_redirect = m.locals.add(ValType::I32);
        let is_rename = m.locals.add(ValType::I32);
        let mut memory_backup = Vec::new();
        for _ in 0..redirect_id.len() {
            memory_backup.push(m.locals.add(pointer_type));
        }
        let redirect_canisters = [
            Principal::from_slice(&[]),
            Principal::from_text("7hfb6-caaaa-aaaar-qadga-cai").unwrap(),
        ];

        // All functions that require controller permissions or cycles.
        // For simplicity, We mingle all canister methods in a single list.
        // Method names shouldn't overlap.
        let controller_function_names = [
            "create_canister",
            "update_settings",
            "install_code",
            "uninstall_code",
            "canister_status",
            "stop_canister",
            "start_canister",
            "delete_canister",
            "list_canister_snapshots",
            "take_canister_snapshot",
            "load_canister_snapshot",
            "delete_canister_snapshot",
            // These functions doesn't require controller permissions, but needs cycles
            "sign_with_ecdsa",
            "sign_with_schnorr",
            "http_request", // Will be renamed to "_ttp_request", because the name conflicts with the http serving endpoint.
            "_ttp_request", // need to redirect renamed function as well, because the second time we see this function, it's already renamed in memory
            // methods from evm canister
            "eth_call",
            "eth_feeHistory",
            "eth_getBlockByNumber",
            "eth_getLogs",
            "eth_getTransactionCount",
            "eth_getTransactionReceipt",
            "eth_sendRawTransaction",
            "request",
        ];

        let mut builder = FunctionBuilder::new(
            &mut m.types,
            &[
                pointer_type,
                pointer_type,
                pointer_type,
                pointer_type,
                pointer_type,
                pointer_type,
                pointer_type,
                pointer_type,
            ],
            &[],
        );

        builder
            .func_body()
            .block(None, |checks| {
                let checks_id = checks.id();
                // Check if callee address is from redirect_canisters
                checks
                    .block(None, |id_check| {
                        check_list(
                            memory,
                            id_check,
                            no_redirect,
                            callee_size,
                            callee_src,
                            None,
                            &redirect_canisters
                                .iter()
                                .map(|p| p.as_slice())
                                .collect::<Vec<_>>(),
                            wasm64,
                        );
                    })
                    .local_get(no_redirect)
                    .br_if(checks_id);
                // Callee address matches, check method name is in the list
                check_list(
                    memory,
                    checks,
                    no_redirect,
                    name_size,
                    name_src,
                    Some(is_rename),
                    &controller_function_names
                        .iter()
                        .map(|s| s.as_bytes())
                        .collect::<Vec<_>>(),
                    wasm64,
                );
            })
            .local_get(no_redirect)
            .if_else(
                None,
                |block| {
                    // Put all the args back on stack and call call_new without redirecting
                    block
                        .local_get(callee_src)
                        .local_get(callee_size)
                        .local_get(name_src)
                        .local_get(name_size)
                        .local_get(arg5)
                        .local_get(arg6)
                        .local_get(arg7)
                        .local_get(arg8)
                        .call(old_call_new);
                },
                |block| {
                    // Save current memory starting from address 0 into local variables
                    for (address, backup_var) in memory_backup.iter().enumerate() {
                        match wasm64 {
                            true => {
                                block
                                    .i64_const(address as i64)
                                    .load(
                                        memory,
                                        LoadKind::I64_8 {
                                            kind: ExtendedLoad::ZeroExtend,
                                        },
                                        MemArg {
                                            offset: 0,
                                            align: 1,
                                        },
                                    )
                                    .local_set(*backup_var);
                            }
                            false => {
                                block
                                    .i32_const(address as i32)
                                    .load(
                                        memory,
                                        LoadKind::I32_8 {
                                            kind: ExtendedLoad::ZeroExtend,
                                        },
                                        MemArg {
                                            offset: 0,
                                            align: 1,
                                        },
                                    )
                                    .local_set(*backup_var);
                            }
                        }
                    }

                    // Write the canister id into memory at address 0
                    for (address, byte) in redirect_id.iter().enumerate() {
                        match wasm64 {
                            true => {
                                block
                                    .i64_const(address as i64)
                                    .i64_const(*byte as i64)
                                    .store(
                                        memory,
                                        StoreKind::I64_8 { atomic: false },
                                        MemArg {
                                            offset: 0,
                                            align: 1,
                                        },
                                    );
                            }
                            false => {
                                block
                                    .i32_const(address as i32)
                                    .i32_const(*byte as i32)
                                    .store(
                                        memory,
                                        StoreKind::I32_8 { atomic: false },
                                        MemArg {
                                            offset: 0,
                                            align: 1,
                                        },
                                    );
                            }
                        }
                    }
                    block.local_get(is_rename).if_else(
                        None,
                        |then| match wasm64 {
                            true => {
                                then.local_get(name_src).i64_const('_' as i64).store(
                                    memory,
                                    StoreKind::I64_8 { atomic: false },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                );
                            }
                            false => {
                                then.local_get(name_src).i32_const('_' as i32).store(
                                    memory,
                                    StoreKind::I32_8 { atomic: false },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                );
                            }
                        },
                        |_| {},
                    );
                    match wasm64 {
                        true => {
                            block.i64_const(0).i64_const(redirect_id.len() as i64);
                        }
                        false => {
                            block.i32_const(0).i32_const(redirect_id.len() as i32);
                        }
                    }

                    block
                        .local_get(name_src)
                        .local_get(name_size)
                        .local_get(arg5)
                        .local_get(arg6)
                        .local_get(arg7)
                        .local_get(arg8)
                        .call(old_call_new);

                    // Restore old memory
                    for (address, byte) in memory_backup.iter().enumerate() {
                        match wasm64 {
                            true => {
                                block.i64_const(address as i64).local_get(*byte).store(
                                    memory,
                                    StoreKind::I64_8 { atomic: false },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                );
                            }
                            false => {
                                block.i32_const(address as i32).local_get(*byte).store(
                                    memory,
                                    StoreKind::I32_8 { atomic: false },
                                    MemArg {
                                        offset: 0,
                                        align: 1,
                                    },
                                );
                            }
                        }
                    }
                },
            );
        let new_call_new = builder.finish(
            vec![
                callee_src,
                callee_size,
                name_src,
                name_size,
                arg5,
                arg6,
                arg7,
                arg8,
            ],
            &mut m.funcs,
        );
        replacer.add(old_call_new, new_call_new);
    }
}

/// Get the FuncionId of a system API in ic0 import.
///
/// If stable_size or stable64_size is not imported, add them to the module.
fn get_ic_func_id(m: &mut Module, method: &str) -> Option<FunctionId> {
    match m.imports.find("ic0", method) {
        Some(id) => match m.imports.get(id).kind {
            ImportKind::Function(func_id) => Some(func_id),
            _ => unreachable!(),
        },
        None => {
            let ty = match method {
                "stable_size" => Some(m.types.add(&[], &[ValType::I32])),
                "stable64_size" => Some(m.types.add(&[], &[ValType::I64])),
                _ => None,
            };
            match ty {
                Some(ty) => {
                    let func_id = m.add_import_func("ic0", method, ty).0;
                    Some(func_id)
                }
                None => None,
            }
        }
    }
}
