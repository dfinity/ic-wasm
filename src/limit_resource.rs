use crate::{utils::*, Error};
use walrus::ir::*;
use walrus::*;

pub struct Config {
    pub remove_cycles_add: bool,
    pub limit_stable_memory_page: Option<u32>,
    pub playground_canister_id: Option<ic_types::Principal>,
}

#[derive(Copy, Clone)]
struct CyclesAdd {
    cycles_add: FunctionId,
    old_cycles_add128: FunctionId,
    new_cycles_add128: FunctionId,
}
#[derive(Copy, Clone)]
struct StableGrow {
    old_grow: FunctionId,
    new_grow: FunctionId,
    old_grow64: FunctionId,
    new_grow64: FunctionId,
}

#[derive(Copy, Clone)]
struct CallNew {
    old_call_new: FunctionId,
    new_call_new: FunctionId,
}

struct Replacer {
    cycles_add: Option<CyclesAdd>,
    stable_grow: Option<StableGrow>,
    call_new: Option<CallNew>,
}
impl VisitorMut for Replacer {
    fn visit_instr_mut(&mut self, instr: &mut Instr, _: &mut InstrLocId) {
        if let Instr::Call(walrus::ir::Call { func }) = instr {
            if let Some(ids) = &self.cycles_add {
                if *func == ids.cycles_add {
                    *instr = Drop {}.into();
                    return;
                } else if *func == ids.old_cycles_add128 {
                    *instr = Call {
                        func: ids.new_cycles_add128,
                    }
                    .into();
                    return;
                }
            }
            if let Some(ids) = &self.stable_grow {
                if *func == ids.old_grow {
                    *instr = Call { func: ids.new_grow }.into();
                    return;
                } else if *func == ids.old_grow64 {
                    *instr = Call {
                        func: ids.new_grow64,
                    }
                    .into();
                    return;
                }
            }
            if let Some(ids) = &self.call_new {
                if *func == ids.old_call_new {
                    *instr = Call {
                        func: ids.new_call_new,
                    }
                    .into();
                }
            }
        }
    }
}

pub fn limit_resource(wasm: &[u8], config: &Config) -> Result<Vec<u8>, Error> {
    let mut m = walrus::ModuleConfig::new()
        .parse(wasm)
        .map_err(|e| Error::WASM(format!("Could not parse the data as WASM module. {}", e)))?;
    limit_resource_(&mut m, config);
    Ok(m.emit_wasm())
}

pub fn limit_resource_(m: &mut Module, config: &Config) {
    let has_cycles_add = m
        .imports
        .find("ic0", "call_cycles_add")
        .or_else(|| m.imports.find("ic0", "call_cycles_add128"))
        .is_some();
    let cycles_add = if has_cycles_add && config.remove_cycles_add {
        let cycles_add = get_ic_func_id(m, "call_cycles_add");
        let old_cycles_add128 = get_ic_func_id(m, "call_cycles_add128");
        let new_cycles_add128 = make_cycles_add128(m);
        Some(CyclesAdd {
            cycles_add,
            old_cycles_add128,
            new_cycles_add128,
        })
    } else {
        None
    };
    let has_grow = m
        .imports
        .find("ic0", "stable_grow")
        .or_else(|| m.imports.find("ic0", "stable64_grow"))
        .is_some();
    let stable_grow = match (has_grow, config.limit_stable_memory_page) {
        (true, Some(limit)) => {
            let old_grow = get_ic_func_id(m, "stable_grow");
            let new_grow = make_grow_func(m, limit as i32);
            let old_grow64 = get_ic_func_id(m, "stable64_grow");
            let new_grow64 = make_grow64_func(m, limit as i64);
            Some(StableGrow {
                old_grow,
                new_grow,
                old_grow64,
                new_grow64,
            })
        }
        (_, _) => None,
    };
    let call_new = m
        .imports
        .find("ic0", "call_new")
        .and(config.playground_canister_id.as_ref())
        .map(|redirect_id| {
            let old_call_new = get_ic_func_id(m, "call_new");
            let new_call_new = make_redirect_call_new(m, redirect_id.as_slice());
            CallNew {
                old_call_new,
                new_call_new,
            }
        });

    m.funcs.iter_local_mut().for_each(|(id, func)| {
        if let Some(ids) = &cycles_add {
            if id == ids.new_cycles_add128 {
                return;
            }
        }
        if let Some(ids) = &stable_grow {
            if id == ids.new_grow || id == ids.new_grow64 {
                return;
            }
        }
        if let Some(ids) = &call_new {
            if id == ids.new_call_new {
                return;
            }
        }
        dfs_pre_order_mut(
            &mut Replacer {
                cycles_add,
                stable_grow,
                call_new,
            },
            func,
            func.entry_block(),
        );
    });
}

fn make_cycles_add128(m: &mut Module) -> FunctionId {
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64, ValType::I64], &[]);
    let high = m.locals.add(ValType::I64);
    let low = m.locals.add(ValType::I64);
    builder
        .func_body()
        .local_get(high)
        .local_get(low)
        .drop()
        .drop();
    builder.finish(vec![high, low], &mut m.funcs)
}
fn make_grow_func(m: &mut Module, limit: i32) -> FunctionId {
    let size = get_ic_func_id(m, "stable_size");
    let grow = get_ic_func_id(m, "stable_grow");
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I32], &[ValType::I32]);
    let requested = m.locals.add(ValType::I32);
    builder
        .func_body()
        .call(size)
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
                else_.local_get(requested).call(grow);
            },
        );
    builder.finish(vec![requested], &mut m.funcs)
}

fn make_grow64_func(m: &mut Module, limit: i64) -> FunctionId {
    let size = get_ic_func_id(m, "stable64_size");
    let grow = get_ic_func_id(m, "stable64_grow");
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64], &[ValType::I64]);
    let requested = m.locals.add(ValType::I64);
    builder
        .func_body()
        .call(size)
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
                else_.local_get(requested).call(grow);
            },
        );
    builder.finish(vec![requested], &mut m.funcs)
}
fn make_redirect_call_new(m: &mut Module, redirect_id: &[u8]) -> FunctionId {
    // Specify the same args as `call_new` so that WASM will correctly check mismatching args
    let callee_src = m.locals.add(ValType::I32);
    let callee_size = m.locals.add(ValType::I32);
    let name_src = m.locals.add(ValType::I32);
    let name_size = m.locals.add(ValType::I32);
    let arg5 = m.locals.add(ValType::I32);
    let arg6 = m.locals.add(ValType::I32);
    let arg7 = m.locals.add(ValType::I32);
    let arg8 = m.locals.add(ValType::I32);
    let call_new = get_ic_func_id(m, "call_new");

    let memory = get_memory_id(m);

    // Scratch variables
    let no_redirect = m.locals.add(ValType::I32);
    let mut memory_backup = Vec::new();
    for _ in 0..redirect_id.len() {
        memory_backup.push(m.locals.add(ValType::I32));
    }

    // All management canister functions that require controller permissions
    // The following wasm code assumes that this list is non-empty
    let controller_function_names = [
        "create_canister",
        "update_settings",
        "install_code",
        "uninstall_code",
        "canister_status",
        "stop_canister",
        "start_canister",
        "delete_canister",
    ];

    let mut builder = FunctionBuilder::new(
        &mut m.types,
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
    );

    builder
        .func_body()
        .block(None, |checks| {
            let checks_id = checks.id();

            // Check that callee address is empty
            checks
                .local_get(callee_size)
                .i32_const(0)
                .binop(BinaryOp::I32Ne)
                .local_tee(no_redirect)
                .br_if(checks_id);

            // Check if the function name is any of the ones to be redirected
            for func_name in controller_function_names {
                checks.block(None, |name_check| {
                    let name_check_id = name_check.id();
                    name_check
                        // Check that name_size is the same length as the function name
                        .local_get(name_size)
                        .i32_const(func_name.len() as i32)
                        .binop(BinaryOp::I32Ne)
                        .br_if(name_check_id);

                    // Load the string at name_src onto the stack and compare it to the function name
                    for i in 0..func_name.len() {
                        name_check.local_get(name_src).load(
                            memory,
                            LoadKind::I32_8 {
                                kind: ExtendedLoad::SignExtend,
                            },
                            MemArg {
                                offset: i as u32,
                                align: 1,
                            },
                        );
                    }
                    for c in func_name.chars().rev() {
                        name_check
                            .i32_const(c as i32)
                            .binop(BinaryOp::I32Ne)
                            .br_if(name_check_id);
                    }
                    // Function names were equal, so skip all remaining checks and redirect
                    name_check.i32_const(0).local_set(no_redirect).br(checks_id);
                });
            }

            // None of the function names matched
            checks.i32_const(1).local_set(no_redirect);
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
                    .call(call_new);
            },
            |block| {
                // Save current memory starting from address 0 into local variables
                for (address, backup_var) in memory_backup.iter().enumerate() {
                    block
                        .i32_const(address as i32)
                        .load(
                            memory,
                            LoadKind::I32_8 {
                                kind: ExtendedLoad::SignExtend,
                            },
                            MemArg {
                                offset: 0,
                                align: 1,
                            },
                        )
                        .local_set(*backup_var);
                }

                // Write the canister id into memory at address 0
                for (address, byte) in redirect_id.iter().enumerate() {
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

                block
                    .i32_const(0)
                    .i32_const(redirect_id.len() as i32)
                    .local_get(name_src)
                    .local_get(name_size)
                    .local_get(arg5)
                    .local_get(arg6)
                    .local_get(arg7)
                    .local_get(arg8)
                    .call(call_new);

                // Restore old memory
                for (address, byte) in memory_backup.iter().enumerate() {
                    block.i32_const(address as i32).local_get(*byte).store(
                        memory,
                        StoreKind::I32_8 { atomic: false },
                        MemArg {
                            offset: 0,
                            align: 1,
                        },
                    );
                }
            },
        );
    builder.finish(
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
    )
}
