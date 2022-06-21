use crate::utils::*;
use walrus::ir::*;
use walrus::*;

pub fn replace_cycles_add_with_drop(m: &mut Module) {
    match (
        m.imports.find("ic0", "call_cycles_add"),
        m.imports.find("ic0", "call_cycles_add128"),
    ) {
        (None, None) => (),
        (_, _) => {
            struct Replacer {
                cycles_add: FunctionId,
                old_cycles_add128: FunctionId,
                new_cycles_add128: FunctionId,
            }
            impl VisitorMut for Replacer {
                fn visit_instr_mut(&mut self, instr: &mut Instr, _instr_loc: &mut InstrLocId) {
                    if let Instr::Call(walrus::ir::Call { func }) = instr {
                        if *func == self.cycles_add {
                            *instr = Drop {}.into();
                        } else if *func == self.old_cycles_add128 {
                            *instr = Call {
                                func: self.new_cycles_add128,
                            }
                            .into();
                        }
                    }
                }
            }
            let cycles_add = get_ic_func_id(m, "call_cycles_add");
            let old_cycles_add128 = get_ic_func_id(m, "call_cycles_add128");
            let new_cycles_add128 = make_cycles_add128(m);
            m.funcs.iter_local_mut().for_each(|(id, func)| {
                if id != new_cycles_add128 {
                    dfs_pre_order_mut(
                        &mut Replacer {
                            cycles_add,
                            old_cycles_add128,
                            new_cycles_add128,
                        },
                        func,
                        func.entry_block(),
                    );
                }
            });
        }
    }
}
pub fn limit_stable_memory_page(m: &mut Module, limit: u32) {
    match (
        m.imports.find("ic0", "stable_grow"),
        m.imports.find("ic0", "stable64_grow"),
    ) {
        (None, None) => (),
        (_, _) => {
            struct Replacer {
                old_grow: FunctionId,
                new_grow: FunctionId,
                old_grow64: FunctionId,
                new_grow64: FunctionId,
            }
            impl VisitorMut for Replacer {
                fn visit_call_mut(&mut self, call: &mut Call) {
                    if call.func == self.old_grow {
                        *call = Call {
                            func: self.new_grow,
                        };
                    } else if call.func == self.old_grow64 {
                        *call = Call {
                            func: self.new_grow64,
                        };
                    }
                }
            }

            let old_grow = get_ic_func_id(m, "stable_grow");
            let new_grow = make_grow_func(m, limit as i32);
            let old_grow64 = get_ic_func_id(m, "stable64_grow");
            let new_grow64 = make_grow64_func(m, limit as i64);
            m.funcs.iter_local_mut().for_each(|(id, func)| {
                if id != new_grow && id != new_grow64 {
                    dfs_pre_order_mut(
                        &mut Replacer {
                            old_grow,
                            new_grow,
                            old_grow64,
                            new_grow64,
                        },
                        func,
                        func.entry_block(),
                    );
                }
            });
        }
    }
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
