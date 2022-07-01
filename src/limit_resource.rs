use crate::utils::*;
use walrus::ir::*;
use walrus::*;

pub struct Config {
    pub remove_cycles_add: bool,
    pub limit_stable_memory_page: Option<u32>,
    pub redirect_create_canister: bool,
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
                    *instr = Call { func: ids.new_call_new }.into();
                    return;
                }
            }
        }
    }
}

pub fn limit_resource(m: &mut Module, config: &Config) {
    if is_motoko_canister(m) {
        let ids = get_motoko_wasm_data_sections(m);
        for (id, mut module) in ids.into_iter() {
            limit_resource(&mut module, config);
            let blob = encode_module_as_data_section(module);
            m.data.get_mut(id).value = blob;
        }
    }
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
        .and_then(|_| {
            let old_call_new = get_ic_func_id(m, "call_new");
            let new_call_new = make_redirect_call_new(m);
            Some(CallNew {
                old_call_new,
                new_call_new,
            })
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
fn make_redirect_call_new(m: &mut Module) -> FunctionId {
    let mut builder = FunctionBuilder::new(&mut m.types,
        &[ValType::I64, ValType::I64, ValType::I64, ValType::I64, ValType::I64, ValType::I64, ValType::I64, ValType::I64],
        &[]);
    // FIXME
    builder
        .func_body()
        .drop()
        .drop();
    builder.finish(vec![], &mut m.funcs)
}
