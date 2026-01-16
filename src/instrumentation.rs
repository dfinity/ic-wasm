use walrus::ir::*;
use walrus::*;

use crate::utils::*;
use std::collections::HashSet;

const METADATA_SIZE: i32 = 24;
const DEFAULT_PAGE_LIMIT: i32 = 16 * 256; // 256M
const LOG_ITEM_SIZE: i32 = 12;
const MAX_ITEMS_PER_QUERY: i32 = 174758; // (2M - 40) / LOG_ITEM_SIZE;

struct InjectionPoint {
    position: usize,
    cost: i64,
    kind: InjectionKind,
}
impl InjectionPoint {
    fn new() -> Self {
        InjectionPoint {
            position: 0,
            cost: 0,
            kind: InjectionKind::Static,
        }
    }
}

struct Variables {
    total_counter: GlobalId,
    log_size: GlobalId,
    page_size: GlobalId,
    is_init: GlobalId,
    is_entry: GlobalId,
    dynamic_counter_func: FunctionId,
    dynamic_counter64_func: FunctionId,
}

pub struct Config {
    pub trace_only_funcs: Vec<String>,
    pub start_address: Option<i64>,
    pub page_limit: Option<i32>,
    pub stub_wasi: bool,
}
impl Config {
    pub fn is_preallocated(&self) -> bool {
        self.start_address.is_some()
    }
    pub fn log_start_address(&self) -> i64 {
        self.start_address.unwrap_or(0) + METADATA_SIZE as i64
    }
    pub fn metadata_start_address(&self) -> i64 {
        self.start_address.unwrap_or(0)
    }
    pub fn page_limit(&self) -> i64 {
        i64::from(
            self.page_limit
                .map(|x| x - 1)
                .unwrap_or(DEFAULT_PAGE_LIMIT - 1),
        ) // minus 1 because of metadata
    }
}

/// When trace_only_funcs is not empty, counting and tracing is only enabled for those listed functions per update call.
/// TODO: doesn't handle recursive entry functions. Need to create a wrapper for the recursive entry function.
pub fn instrument(m: &mut Module, config: Config) -> Result<(), String> {
    if config.stub_wasi {
        stub_wasi_imports(m);
    }
    let mut trace_only_ids = HashSet::new();
    for name in config.trace_only_funcs.iter() {
        let id = match m.funcs.by_name(name) {
            Some(id) => id,
            None => return Err(format!("func \"{name}\" not found")),
        };
        trace_only_ids.insert(id);
    }
    let is_partial_tracing = !trace_only_ids.is_empty();
    let func_cost = FunctionCost::new(m);
    let total_counter =
        m.globals
            .add_local(ValType::I64, true, false, ConstExpr::Value(Value::I64(0)));
    let log_size = m
        .globals
        .add_local(ValType::I32, true, false, ConstExpr::Value(Value::I32(0)));
    let page_size = m
        .globals
        .add_local(ValType::I32, true, false, ConstExpr::Value(Value::I32(0)));
    let is_init = m
        .globals
        .add_local(ValType::I32, true, false, ConstExpr::Value(Value::I32(1)));
    let is_entry = m
        .globals
        .add_local(ValType::I32, true, false, ConstExpr::Value(Value::I32(0)));
    let opt_init = if is_partial_tracing {
        Some(is_init)
    } else {
        None
    };
    let dynamic_counter_func = make_dynamic_counter(m, total_counter, &opt_init);
    let dynamic_counter64_func = make_dynamic_counter64(m, total_counter, &opt_init);
    let vars = Variables {
        total_counter,
        log_size,
        is_init,
        is_entry,
        dynamic_counter_func,
        dynamic_counter64_func,
        page_size,
    };

    for (id, func) in m.funcs.iter_local_mut() {
        if id != dynamic_counter_func && id != dynamic_counter64_func {
            inject_metering(
                func,
                func.entry_block(),
                &vars,
                &func_cost,
                is_partial_tracing,
            );
        }
    }
    let writer = make_stable_writer(m, &vars, &config);
    let printer = make_printer(m, &vars, writer);
    for (id, func) in m.funcs.iter_local_mut() {
        if id != printer
            && id != writer
            && id != dynamic_counter_func
            && id != dynamic_counter64_func
        {
            let is_partial_tracing = trace_only_ids.contains(&id);
            inject_profiling_prints(&m.types, printer, id, func, is_partial_tracing, &vars);
        }
    }
    if !is_partial_tracing {
        //inject_start(m, vars.is_init);
        inject_init(m, vars.is_init);
    }
    // Persist globals
    inject_pre_upgrade(m, &vars, &config);
    inject_post_upgrade(m, &vars, &config);

    inject_canister_methods(m, &vars);
    let leb = make_leb128_encoder(m);
    make_stable_getter(m, &vars, leb, &config);
    make_getter(m, &vars);
    make_toggle_func(m, "__toggle_tracing", vars.is_init);
    make_toggle_func(m, "__toggle_entry", vars.is_entry);
    let name = make_name_section(m);
    m.customs.add(name);
    Ok(())
}

fn inject_metering(
    func: &mut LocalFunction,
    start: InstrSeqId,
    vars: &Variables,
    func_cost: &FunctionCost,
    is_partial_tracing: bool,
) {
    use InjectionKind::*;
    let mut stack = vec![start];
    while let Some(seq_id) = stack.pop() {
        let seq = func.block(seq_id);
        // Finding injection points
        let mut injection_points = vec![];
        let mut curr = InjectionPoint::new();
        // each function has at least a unit cost
        if seq_id == start {
            curr.cost += 1;
        }
        for (pos, (instr, _)) in seq.instrs.iter().enumerate() {
            curr.position = pos;
            match instr {
                Instr::Block(Block { seq }) | Instr::Loop(Loop { seq }) => {
                    match func.block(*seq).ty {
                        InstrSeqType::Simple(Some(_)) => curr.cost += instr_cost(instr),
                        InstrSeqType::Simple(None) => (),
                        InstrSeqType::MultiValue(_) => unreachable!("Multivalue not supported"),
                    }
                    stack.push(*seq);
                    injection_points.push(curr);
                    curr = InjectionPoint::new();
                }
                Instr::IfElse(IfElse {
                    consequent,
                    alternative,
                }) => {
                    curr.cost += instr_cost(instr);
                    stack.push(*consequent);
                    stack.push(*alternative);
                    injection_points.push(curr);
                    curr = InjectionPoint::new();
                }
                Instr::Br(_) | Instr::BrIf(_) | Instr::BrTable(_) => {
                    // br always points to a block, so we don't need to push the br block to stack for traversal
                    curr.cost += instr_cost(instr);
                    injection_points.push(curr);
                    curr = InjectionPoint::new();
                }
                Instr::Return(_) | Instr::Unreachable(_) => {
                    curr.cost += instr_cost(instr);
                    injection_points.push(curr);
                    curr = InjectionPoint::new();
                }
                Instr::Call(Call { func }) => {
                    curr.cost += instr_cost(instr);
                    match func_cost.get_cost(*func) {
                        Some((cost, InjectionKind::Static)) => curr.cost += cost,
                        Some((cost, kind @ InjectionKind::Dynamic))
                        | Some((cost, kind @ InjectionKind::Dynamic64)) => {
                            curr.cost += cost;
                            let dynamic = InjectionPoint {
                                position: pos,
                                cost: 0,
                                kind,
                            };
                            injection_points.push(dynamic);
                        }
                        None => {}
                    }
                }
                Instr::MemoryFill(_)
                | Instr::MemoryCopy(_)
                | Instr::MemoryInit(_)
                | Instr::TableCopy(_)
                | Instr::TableInit(_) => {
                    curr.cost += instr_cost(instr);
                    let dynamic = InjectionPoint {
                        position: pos,
                        cost: 0,
                        kind: InjectionKind::Dynamic,
                    };
                    injection_points.push(dynamic);
                }
                _ => {
                    curr.cost += instr_cost(instr);
                }
            }
        }
        injection_points.push(curr);
        // Reconstruct instructions
        let injection_points = injection_points
            .iter()
            .filter(|point| point.cost > 0 || point.kind != Static);
        let mut builder = func.builder_mut().instr_seq(seq_id);
        let original = builder.instrs_mut();
        let mut instrs = vec![];
        let mut last_injection_position = 0;
        for point in injection_points {
            instrs.extend_from_slice(&original[last_injection_position..point.position]);
            // injection happens one instruction before the injection_points, so the cost contains
            // the control flow instruction.
            match point.kind {
                Static => {
                    #[rustfmt::skip]
                    instrs.extend_from_slice(&[
                        (GlobalGet { global: vars.total_counter }.into(), Default::default()),
                        (Const { value: Value::I64(point.cost) }.into(), Default::default()),
                    ]);
                    if is_partial_tracing {
                        #[rustfmt::skip]
                        instrs.extend_from_slice(&[
                            (GlobalGet { global: vars.is_init }.into(), Default::default()),
                            (Const { value: Value::I32(1) }.into(), Default::default()),
                            (Binop { op: BinaryOp::I32Xor }.into(), Default::default()),
                            (Unop { op: UnaryOp::I64ExtendUI32 }.into(), Default::default()),
                            (Binop { op: BinaryOp::I64Mul }.into(), Default::default()),
                        ]);
                    }
                    #[rustfmt::skip]
                    instrs.extend_from_slice(&[
                        (Binop { op: BinaryOp::I64Add }.into(), Default::default()),
                        (GlobalSet { global: vars.total_counter }.into(), Default::default()),
                    ]);
                }
                Dynamic => {
                    // Assume top of the stack is the i32 size parameter
                    #[rustfmt::skip]
                    instrs.push((Call { func: vars.dynamic_counter_func }.into(), Default::default()));
                }
                Dynamic64 => {
                    #[rustfmt::skip]
                    instrs.push((Call { func: vars.dynamic_counter64_func }.into(), Default::default()));
                }
            };
            last_injection_position = point.position;
        }
        instrs.extend_from_slice(&original[last_injection_position..]);
        *original = instrs;
    }
}

fn inject_profiling_prints(
    types: &ModuleTypes,
    printer: FunctionId,
    id: FunctionId,
    func: &mut LocalFunction,
    is_partial_tracing: bool,
    vars: &Variables,
) {
    // Put the original function body inside a block, so that if the code
    // use br_if/br_table to exit the function, we can still output the exit signal.
    let start_id = func.entry_block();
    let original_block = func.block_mut(start_id);
    let start_instrs = original_block.instrs.split_off(0);
    let start_ty = match original_block.ty {
        InstrSeqType::MultiValue(id) => {
            let valtypes = types.results(id);
            InstrSeqType::Simple(match valtypes.len() {
                0 => None,
                1 => Some(valtypes[0]),
                _ => unreachable!("Multivalue return not supported"),
            })
        }
        // top-level block is using the function signature
        InstrSeqType::Simple(_) => unreachable!(),
    };
    let mut inner_start = func.builder_mut().dangling_instr_seq(start_ty);
    *(inner_start.instrs_mut()) = start_instrs;
    let inner_start_id = inner_start.id();
    let mut start_builder = func.builder_mut().func_body();
    if is_partial_tracing {
        start_builder.i32_const(0).global_set(vars.is_init);
    }
    start_builder
        .i32_const(id.index() as i32)
        .call(printer)
        .instr(Block {
            seq: inner_start_id,
        })
        // TOOD fix when id == 0
        .i32_const(-(id.index() as i32))
        .call(printer);
    // TODO this only works for non-recursive entry function
    if is_partial_tracing {
        start_builder.i32_const(1).global_set(vars.is_init);
    }
    let mut stack = vec![inner_start_id];
    while let Some(seq_id) = stack.pop() {
        let mut builder = func.builder_mut().instr_seq(seq_id);
        let original = builder.instrs_mut();
        let mut instrs = vec![];
        for (instr, loc) in original.iter() {
            match instr {
                Instr::Block(Block { seq }) | Instr::Loop(Loop { seq }) => {
                    stack.push(*seq);
                    instrs.push((instr.clone(), *loc));
                }
                Instr::IfElse(IfElse {
                    consequent,
                    alternative,
                }) => {
                    stack.push(*alternative);
                    stack.push(*consequent);
                    instrs.push((instr.clone(), *loc));
                }
                Instr::Return(_) => {
                    instrs.push((
                        Instr::Br(Br {
                            block: inner_start_id,
                        }),
                        *loc,
                    ));
                }
                // redirect br,br_if,br_table to inner seq id
                Instr::Br(Br { block }) if *block == start_id => {
                    instrs.push((
                        Instr::Br(Br {
                            block: inner_start_id,
                        }),
                        *loc,
                    ));
                }
                Instr::BrIf(BrIf { block }) if *block == start_id => {
                    instrs.push((
                        Instr::BrIf(BrIf {
                            block: inner_start_id,
                        }),
                        *loc,
                    ));
                }
                Instr::BrTable(BrTable { blocks, default }) => {
                    let mut blocks = blocks.clone();
                    for i in 0..blocks.len() {
                        if let Some(id) = blocks.get_mut(i) {
                            if *id == start_id {
                                *id = inner_start_id
                            };
                        }
                    }
                    let default = if *default == start_id {
                        inner_start_id
                    } else {
                        *default
                    };
                    instrs.push((Instr::BrTable(BrTable { blocks, default }), *loc));
                }
                _ => instrs.push((instr.clone(), *loc)),
            }
        }
        *original = instrs;
    }
}

fn make_dynamic_counter(
    m: &mut Module,
    total_counter: GlobalId,
    opt_init: &Option<GlobalId>,
) -> FunctionId {
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I32], &[ValType::I32]);
    let size = m.locals.add(ValType::I32);
    let mut seq = builder.func_body();
    seq.local_get(size);
    if let Some(is_init) = opt_init {
        seq.global_get(*is_init)
            .i32_const(1)
            .binop(BinaryOp::I32Xor)
            .binop(BinaryOp::I32Mul);
    }
    seq.unop(UnaryOp::I64ExtendUI32)
        .global_get(total_counter)
        .binop(BinaryOp::I64Add)
        .global_set(total_counter)
        .local_get(size);
    builder.finish(vec![size], &mut m.funcs)
}
fn make_dynamic_counter64(
    m: &mut Module,
    total_counter: GlobalId,
    opt_init: &Option<GlobalId>,
) -> FunctionId {
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I64], &[ValType::I64]);
    let size = m.locals.add(ValType::I64);
    let mut seq = builder.func_body();
    seq.local_get(size);
    if let Some(is_init) = opt_init {
        seq.global_get(*is_init)
            .i32_const(1)
            .binop(BinaryOp::I32Xor)
            .unop(UnaryOp::I64ExtendUI32)
            .binop(BinaryOp::I64Mul);
    }
    seq.global_get(total_counter)
        .binop(BinaryOp::I64Add)
        .global_set(total_counter)
        .local_get(size);
    builder.finish(vec![size], &mut m.funcs)
}
fn make_stable_writer(m: &mut Module, vars: &Variables, config: &Config) -> FunctionId {
    let writer = get_ic_func_id(m, "stable64_write");
    let grow = get_ic_func_id(m, "stable64_grow");
    let mut builder = FunctionBuilder::new(
        &mut m.types,
        &[ValType::I64, ValType::I64, ValType::I64],
        &[],
    );
    let start_address = config.log_start_address();
    let size_limit = config.page_limit() * 65536;
    let is_preallocated = config.is_preallocated();
    let offset = m.locals.add(ValType::I64);
    let src = m.locals.add(ValType::I64);
    let size = m.locals.add(ValType::I64);
    builder
        .func_body()
        .local_get(offset)
        .local_get(size)
        .binop(BinaryOp::I64Add);
    if is_preallocated {
        builder.func_body().i64_const(size_limit);
    } else {
        builder
            .func_body()
            .global_get(vars.page_size)
            .i32_const(65536)
            .binop(BinaryOp::I32Mul)
            .i32_const(METADATA_SIZE)
            .binop(BinaryOp::I32Sub)
            // SI because it can be negative
            .unop(UnaryOp::I64ExtendSI32);
    }
    builder
        .func_body()
        .binop(BinaryOp::I64GtS)
        .if_else(
            None,
            |then| {
                if is_preallocated {
                    then.return_();
                } else {
                    // This assumes user code doesn't use stable memory
                    then.global_get(vars.page_size)
                        .i32_const(DEFAULT_PAGE_LIMIT)
                        .binop(BinaryOp::I32GtS) // trace > default_page_limit
                        .if_else(
                            None,
                            |then| {
                                then.return_();
                            },
                            |else_| {
                                else_
                                    .i64_const(1)
                                    .call(grow)
                                    .drop()
                                    .global_get(vars.page_size)
                                    .i32_const(1)
                                    .binop(BinaryOp::I32Add)
                                    .global_set(vars.page_size);
                            },
                        );
                }
            },
            |_| {},
        )
        .i64_const(start_address)
        .local_get(offset)
        .binop(BinaryOp::I64Add)
        .local_get(src)
        .local_get(size)
        .call(writer)
        .global_get(vars.log_size)
        .i32_const(1)
        .binop(BinaryOp::I32Add)
        .global_set(vars.log_size);
    builder.finish(vec![offset, src, size], &mut m.funcs)
}

fn make_printer(m: &mut Module, vars: &Variables, writer: FunctionId) -> FunctionId {
    let memory = get_memory_id(m);
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I32], &[]);
    let func_id = m.locals.add(ValType::I32);
    let a = m.locals.add(ValType::I32);
    let b = m.locals.add(ValType::I64);
    builder.func_body().global_get(vars.is_init).if_else(
        None,
        |then| {
            then.return_();
        },
        |else_| {
            #[rustfmt::skip]
            else_
                // backup memory
                .i32_const(0)
                .load(memory, LoadKind::I32 { atomic: false }, MemArg { offset: 0, align: 4})
                .local_set(a)
                .i32_const(4)
                .load(memory, LoadKind::I64 { atomic: false }, MemArg { offset: 0, align: 8})
                .local_set(b)
                // print
                .i32_const(0)
                .local_get(func_id)
                .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
                .i32_const(4)
                .global_get(vars.total_counter)
                .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
                .global_get(vars.log_size)
                .unop(UnaryOp::I64ExtendUI32)
                .i64_const(LOG_ITEM_SIZE as i64)
                .binop(BinaryOp::I64Mul)
                .i64_const(0)
                .i64_const(LOG_ITEM_SIZE as i64)
                .call(writer)
                // restore memory
                .i32_const(0)
                .local_get(a)
                .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
                .i32_const(4)
                .local_get(b)
                .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 });
        },
    );
    builder.finish(vec![func_id], &mut m.funcs)
}
/*
// We can use this function once we have a system memroy for logs.
// Otherwise, we cannot call stable_write in canister_init
fn inject_start(m: &mut Module, is_init: GlobalId) {
    if let Some(id) = m.start {
        let mut builder = get_builder(m, id);
        #[rustfmt::skip]
        builder
            .instr(Const { value: Value::I32(0) })
            .instr(GlobalSet { global: is_init });
    }
}
*/
fn inject_canister_methods(m: &mut Module, vars: &Variables) {
    let methods: Vec<_> = m
        .exports
        .iter()
        .filter_map(|e| match e.item {
            ExportItem::Function(id)
                if e.name != "canister_update __motoko_async_helper"
                    && (e.name.starts_with("canister_update")
                    || e.name.starts_with("canister_query")
                    || e.name.starts_with("canister_composite_query")
                    || e.name.starts_with("canister_heartbeat")
                    // don't clear logs for timer and post_upgrade, as they are trigger by other signals
                    //|| e.name == "canister_global_timer"
                    //|| e.name == "canister_post_upgrade"
                    || e.name == "canister_pre_upgrade") =>
            {
                Some(id)
            }
            _ => None,
        })
        .collect();
    for id in methods.iter() {
        let mut builder = get_builder(m, *id);
        #[rustfmt::skip]
        inject_top(
            &mut builder,
            vec![
                // log_size = is_entry ? log_size : 0
                GlobalGet { global: vars.is_entry }.into(),
                GlobalGet { global: vars.log_size }.into(),
                Binop { op: BinaryOp::I32Mul }.into(),
                GlobalSet { global: vars.log_size }.into(),
            ],
        );
    }
}
fn inject_init(m: &mut Module, is_init: GlobalId) {
    let mut builder = get_or_create_export_func(m, "canister_init");
    // canister_init in Motoko use stable_size to decide if there is stable memory to deserialize.
    // Region initialization in Motoko is also done here.
    // We can only enable profiling at the end of init, otherwise stable.grow breaks this check.
    builder.i32_const(0).global_set(is_init);
}
fn inject_pre_upgrade(m: &mut Module, vars: &Variables, config: &Config) {
    let writer = get_ic_func_id(m, "stable64_write");
    let memory = get_memory_id(m);
    let a = m.locals.add(ValType::I64);
    let b = m.locals.add(ValType::I64);
    let c = m.locals.add(ValType::I64);
    let mut builder = get_or_create_export_func(m, "canister_pre_upgrade");
    #[rustfmt::skip]
    builder
        // backup memory. This is not strictly needed, since it's at the end of pre-upgrade.
        .i32_const(0)
        .load(memory, LoadKind::I64 { atomic: false }, MemArg { offset: 0, align: 8})
        .local_set(a)
        .i32_const(8)
        .load(memory, LoadKind::I64 { atomic: false }, MemArg { offset: 0, align: 8})
        .local_set(b)
        .i32_const(16)
        .load(memory, LoadKind::I64 { atomic: false }, MemArg { offset: 0, align: 8})
        .local_set(c)
        // persist globals
        .i32_const(0)
        .global_get(vars.total_counter)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(8)
        .global_get(vars.log_size)
        .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
        .i32_const(12)
        .global_get(vars.page_size)
        .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
        .i32_const(16)
        .global_get(vars.is_init)
        .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
        .i32_const(20)
        .global_get(vars.is_entry)
        .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4 })
        .i64_const(config.metadata_start_address())
        .i64_const(0)
        .i64_const(METADATA_SIZE as i64)
        .call(writer)
        // restore memory
        .i32_const(0)
        .local_get(a)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(8)
        .local_get(b)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(16)
        .local_get(c)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 });
}
fn inject_post_upgrade(m: &mut Module, vars: &Variables, config: &Config) {
    let reader = get_ic_func_id(m, "stable64_read");
    let memory = get_memory_id(m);
    let a = m.locals.add(ValType::I64);
    let b = m.locals.add(ValType::I64);
    let c = m.locals.add(ValType::I64);
    let mut builder = get_or_create_export_func(m, "canister_post_upgrade");
    #[rustfmt::skip]
    inject_top(&mut builder, vec![
        // backup
        Const { value: Value::I32(0) }.into(),
        Load { memory, kind: LoadKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        LocalSet { local: a }.into(),
        Const { value: Value::I32(8) }.into(),
        Load { memory, kind: LoadKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        LocalSet { local: b }.into(),
        Const { value: Value::I32(16) }.into(),
        Load { memory, kind: LoadKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        LocalSet { local: c }.into(),
        // load from stable memory
        Const { value: Value::I64(0) }.into(),
        Const { value: Value::I64(config.metadata_start_address()) }.into(),
        Const { value: Value::I64(METADATA_SIZE as i64) }.into(),
        Call { func: reader }.into(),
        Const { value: Value::I32(0) }.into(),
        Load { memory, kind: LoadKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        GlobalSet { global: vars.total_counter }.into(),
        Const { value: Value::I32(8) }.into(),
        Load { memory, kind: LoadKind::I32 { atomic: false }, arg: MemArg { offset: 0, align: 4 } }.into(),
        GlobalSet { global: vars.log_size }.into(),
        Const { value: Value::I32(12) }.into(),
        Load { memory, kind: LoadKind::I32 { atomic: false }, arg: MemArg { offset: 0, align: 4 } }.into(),
        GlobalSet { global: vars.page_size }.into(),
        Const { value: Value::I32(16) }.into(),
        Load { memory, kind: LoadKind::I32 { atomic: false }, arg: MemArg { offset: 0, align: 4 } }.into(),
        GlobalSet { global: vars.is_init }.into(),
        Const { value: Value::I32(20) }.into(),
        Load { memory, kind: LoadKind::I32 { atomic: false }, arg: MemArg { offset: 0, align: 4 } }.into(),
        GlobalSet { global: vars.is_entry }.into(),
        // restore
        Const { value: Value::I32(0) }.into(),
        LocalGet { local: a }.into(),
        Store { memory, kind: StoreKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        Const { value: Value::I32(8) }.into(),
        LocalGet { local: b }.into(),
        Store { memory, kind: StoreKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
        Const { value: Value::I32(16) }.into(),
        LocalGet { local: c }.into(),
        Store { memory, kind: StoreKind::I64 { atomic: false }, arg: MemArg { offset: 0, align: 8 } }.into(),
    ]);
}

fn make_stable_getter(m: &mut Module, vars: &Variables, leb: FunctionId, config: &Config) {
    let memory = get_memory_id(m);
    let arg_size = get_ic_func_id(m, "msg_arg_data_size");
    let arg_copy = get_ic_func_id(m, "msg_arg_data_copy");
    let reply_data = get_ic_func_id(m, "msg_reply_data_append");
    let reply = get_ic_func_id(m, "msg_reply");
    let trap = get_ic_func_id(m, "trap");
    let reader = get_ic_func_id(m, "stable64_read");
    let idx = m.locals.add(ValType::I32);
    let len = m.locals.add(ValType::I32);
    let next_idx = m.locals.add(ValType::I32);
    let mut builder = FunctionBuilder::new(&mut m.types, &[], &[]);
    builder.name("__get_profiling".to_string());
    #[rustfmt::skip]
    builder.func_body()
        // allocate 2M of heap memory, it's a query call, the system will give back the memory.
        .memory_size(memory)
        .i32_const(32)
        .binop(BinaryOp::I32LtU)
        .if_else(
            None,
            |then| {
                then
                    .i32_const(32)
                    .memory_grow(memory)
                    .drop();
            },
            |_| {}
        )
        // parse input idx
        .call(arg_size)
        .i32_const(11)
        .binop(BinaryOp::I32Ne)
        .if_else(
            None,
            |then| {
                then.i32_const(0)
                    .i32_const(0)
                    .call(trap);
            },
            |_| {},
        )
        .i32_const(0)
        .i32_const(7)
        .i32_const(4)
        .call(arg_copy)
        .i32_const(0)
        .load(memory, LoadKind::I32 { atomic: false }, MemArg { offset: 0, align: 4})
        .local_set(idx)
        // write header (vec { record { int32; int64 } }, opt int32)
        .i32_const(0)
        .i64_const(0x6c016d034c444944) // "DIDL036d016c"
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(8)
        .i64_const(0x02756e7401750002)  // "02007501746e7502"
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(16)
        .i32_const(0x0200) // "0002"
        .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4})
        .i32_const(0)
        .i32_const(18)
        .call(reply_data)
        // if log_size - idx > MAX_ITEMS_PER_QUERY
        .global_get(vars.log_size)
        .local_get(idx)
        .binop(BinaryOp::I32Sub)
        .local_tee(len)
        .i32_const(MAX_ITEMS_PER_QUERY)
        .binop(BinaryOp::I32GtU)
        .if_else(
            None,
            |then| {
                then.i32_const(MAX_ITEMS_PER_QUERY)
                    .local_set(len)
                    .local_get(idx)
                    .i32_const(MAX_ITEMS_PER_QUERY)
                    .binop(BinaryOp::I32Add)
                    .local_set(next_idx);
            },
            |else_| {
                else_.i32_const(0)
                    .local_set(next_idx);
            },
        )
        .local_get(len)
        .call(leb)
        .i32_const(0)
        .i32_const(5)
        .call(reply_data)
        // read stable logs
        .i64_const(0)
        .i64_const(config.log_start_address())
        .local_get(idx)
        .unop(UnaryOp::I64ExtendUI32)
        .i64_const(LOG_ITEM_SIZE as i64)
        .binop(BinaryOp::I64Mul)
        .binop(BinaryOp::I64Add)
        .local_get(len)
        .unop(UnaryOp::I64ExtendUI32)
        .i64_const(LOG_ITEM_SIZE as i64)
        .binop(BinaryOp::I64Mul)
        .call(reader)
        .i32_const(0)
        .local_get(len)
        .i32_const(LOG_ITEM_SIZE)
        .binop(BinaryOp::I32Mul)
        .call(reply_data)
        // opt next idx
        .local_get(next_idx)
        .unop(UnaryOp::I32Eqz)
        .if_else(
            None,
            |then| {
                then.i32_const(0)
                    .i32_const(0)
                    .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4})
                    .i32_const(0)
                    .i32_const(1)
                    .call(reply_data);
            },
            |else_| {
                else_.i32_const(0)
                    .i32_const(1)
                    .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 1})
                    .i32_const(1)
                    .local_get(next_idx)
                    .store(memory, StoreKind::I32 { atomic: false }, MemArg { offset: 0, align: 4})
                    .i32_const(0)
                    .i32_const(5)
                    .call(reply_data);
            },
        )
        .call(reply);
    let getter = builder.finish(vec![], &mut m.funcs);
    m.exports.add("canister_query __get_profiling", getter);
}
// Generate i32 to 5-byte LEB128 encoding at memory address 0..5
fn make_leb128_encoder(m: &mut Module) -> FunctionId {
    let memory = get_memory_id(m);
    let mut builder = FunctionBuilder::new(&mut m.types, &[ValType::I32], &[]);
    let value = m.locals.add(ValType::I32);
    let mut instrs = builder.func_body();
    for i in 0..5 {
        instrs
            .i32_const(i)
            .local_get(value)
            .i32_const(0x7f)
            .binop(BinaryOp::I32And);
        if i < 4 {
            instrs.i32_const(0x80).binop(BinaryOp::I32Or);
        }
        #[rustfmt::skip]
        instrs
            .store(memory, StoreKind::I32_8 { atomic: false }, MemArg { offset: 0, align: 1 })
            .local_get(value)
            .i32_const(7)
            .binop(BinaryOp::I32ShrU)
            .local_set(value);
    }
    builder.finish(vec![value], &mut m.funcs)
}
fn make_name_section(m: &Module) -> RawCustomSection {
    use candid::Encode;
    let name: Vec<_> = m
        .funcs
        .iter()
        .filter_map(|f| {
            if matches!(f.kind, FunctionKind::Local(_)) {
                use rustc_demangle::demangle;
                let name = f.name.as_ref()?;
                let demangled = format!("{:#}", demangle(name));
                Some((f.id().index() as u16, demangled))
            } else {
                None
            }
        })
        .collect();
    let data = Encode!(&name).unwrap();
    RawCustomSection {
        name: "icp:public name".to_string(),
        data,
    }
}

fn make_getter(m: &mut Module, vars: &Variables) {
    let memory = get_memory_id(m);
    let reply_data = get_ic_func_id(m, "msg_reply_data_append");
    let reply = get_ic_func_id(m, "msg_reply");
    let mut getter = FunctionBuilder::new(&mut m.types, &[], &[]);
    getter.name("__get_cycles".to_string());
    #[rustfmt::skip]
    getter
        .func_body()
        // It's a query call, so we can arbitrarily change the memory without restoring them afterwards.
        .i32_const(0)
        .i64_const(0x007401004c444944)  // "DIDL000174xx" in little endian
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(7)
        .global_get(vars.total_counter)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(0)
        .i32_const(15)
        .call(reply_data)
        .call(reply);
    let getter = getter.finish(vec![], &mut m.funcs);
    m.exports.add("canister_query __get_cycles", getter);
}
fn make_toggle_func(m: &mut Module, name: &str, var: GlobalId) {
    let memory = get_memory_id(m);
    let reply_data = get_ic_func_id(m, "msg_reply_data_append");
    let reply = get_ic_func_id(m, "msg_reply");
    let tmp = m.locals.add(ValType::I64);
    let mut builder = FunctionBuilder::new(&mut m.types, &[], &[]);
    builder.name(name.to_string());
    #[rustfmt::skip]
    builder
        .func_body()
        .global_get(var)
        .i32_const(1)
        .binop(BinaryOp::I32Xor)
        .global_set(var)
        .i32_const(0)
        .load(memory, LoadKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .local_set(tmp)
        .i32_const(0)
        .i64_const(0x4c444944) // "DIDL0000xxxx"
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .i32_const(0)
        .i32_const(6)
        .call(reply_data)
        .i32_const(0)
        .local_get(tmp)
        .store(memory, StoreKind::I64 { atomic: false }, MemArg { offset: 0, align: 8 })
        .call(reply);
    let id = builder.finish(vec![], &mut m.funcs);
    m.exports.add(&format!("canister_update {name}"), id);
}

/// Replace WASI imports with stub functions that return 0 (success) or trap for proc_exit
fn stub_wasi_imports(m: &mut Module) {
    use walrus::FunctionBuilder;

    // Find all WASI imports
    let wasi_imports: Vec<_> = m
        .imports
        .iter()
        .filter(|i| i.module == "wasi_snapshot_preview1")
        .filter_map(|i| {
            if let ImportKind::Function(func_id) = i.kind {
                Some((i.id(), i.name.clone(), func_id))
            } else {
                None
            }
        })
        .collect();

    let memory = m.memories.iter().next().map(|mem| mem.id());

    for (import_id, name, old_func_id) in wasi_imports {
        // Get the function type
        let func = m.funcs.get(old_func_id);
        let ty_id = func.ty();
        let ty = m.types.get(ty_id);
        let params: Vec<_> = ty.params().to_vec();
        let results: Vec<_> = ty.results().to_vec();

        // Create stub function
        let mut builder = FunctionBuilder::new(&mut m.types, &params, &results);
        builder.name(format!("__wasi_{name}_stub"));

        // Create locals for parameters
        let param_locals: Vec<_> = params.iter().map(|t| m.locals.add(*t)).collect();

        match name.as_str() {
            "fd_write" => {
                // fd_write(fd: i32, iovs: i32, iovs_len: i32, nwritten: i32) -> i32
                // Write 0 to nwritten and return 0
                if let Some(mem) = memory {
                    if param_locals.len() >= 4 {
                        builder
                            .func_body()
                            .local_get(param_locals[3]) // nwritten ptr
                            .i32_const(0)
                            .store(
                                mem,
                                StoreKind::I32 { atomic: false },
                                MemArg {
                                    offset: 0,
                                    align: 4,
                                },
                            )
                            .i32_const(0);
                    } else {
                        builder.func_body().i32_const(0);
                    }
                } else {
                    builder.func_body().i32_const(0);
                }
            }
            "fd_read" => {
                // fd_read(fd: i32, iovs: i32, iovs_len: i32, nread: i32) -> i32
                // Write 0 to nread and return 0
                if let Some(mem) = memory {
                    if param_locals.len() >= 4 {
                        builder
                            .func_body()
                            .local_get(param_locals[3]) // nread ptr
                            .i32_const(0)
                            .store(
                                mem,
                                StoreKind::I32 { atomic: false },
                                MemArg {
                                    offset: 0,
                                    align: 4,
                                },
                            )
                            .i32_const(0);
                    } else {
                        builder.func_body().i32_const(0);
                    }
                } else {
                    builder.func_body().i32_const(0);
                }
            }
            "fd_seek" => {
                // fd_seek(fd: i32, offset: i64, whence: i32, newoffset: i32) -> i32
                // Write 0 to newoffset and return 0
                if let Some(mem) = memory {
                    if param_locals.len() >= 4 {
                        builder
                            .func_body()
                            .local_get(param_locals[3]) // newoffset ptr
                            .i64_const(0)
                            .store(
                                mem,
                                StoreKind::I64 { atomic: false },
                                MemArg {
                                    offset: 0,
                                    align: 8,
                                },
                            )
                            .i32_const(0);
                    } else {
                        builder.func_body().i32_const(0);
                    }
                } else {
                    builder.func_body().i32_const(0);
                }
            }
            "fd_close" => {
                // fd_close(fd: i32) -> i32
                // Just return 0 (success)
                builder.func_body().i32_const(0);
            }
            "environ_sizes_get" => {
                // environ_sizes_get(count: i32, buf_size: i32) -> i32
                // Write 0 to both pointers and return 0
                if let Some(mem) = memory {
                    if param_locals.len() >= 2 {
                        builder
                            .func_body()
                            .local_get(param_locals[0]) // count ptr
                            .i32_const(0)
                            .store(
                                mem,
                                StoreKind::I32 { atomic: false },
                                MemArg {
                                    offset: 0,
                                    align: 4,
                                },
                            )
                            .local_get(param_locals[1]) // buf_size ptr
                            .i32_const(0)
                            .store(
                                mem,
                                StoreKind::I32 { atomic: false },
                                MemArg {
                                    offset: 0,
                                    align: 4,
                                },
                            )
                            .i32_const(0);
                    } else {
                        builder.func_body().i32_const(0);
                    }
                } else {
                    builder.func_body().i32_const(0);
                }
            }
            "environ_get" => {
                // environ_get(environ: i32, environ_buf: i32) -> i32
                // Just return 0 (no environment variables)
                builder.func_body().i32_const(0);
            }
            "proc_exit" => {
                // proc_exit(code: i32) -> !
                // Trap unconditionally
                builder.func_body().unreachable();
            }
            _ => {
                // Default: just return 0 for i32 result, or appropriate zero values
                for result in &results {
                    match result {
                        ValType::I32 => {
                            builder.func_body().i32_const(0);
                        }
                        ValType::I64 => {
                            builder.func_body().i64_const(0);
                        }
                        ValType::F32 => {
                            builder.func_body().f32_const(0.0);
                        }
                        ValType::F64 => {
                            builder.func_body().f64_const(0.0);
                        }
                        _ => {}
                    }
                }
            }
        }

        let stub_func_id = builder.finish(param_locals, &mut m.funcs);

        // Replace all calls to old_func_id with stub_func_id
        for (_, func) in m.funcs.iter_local_mut() {
            replace_calls_in_func(func, old_func_id, stub_func_id);
        }

        // Remove the import
        m.imports.delete(import_id);
    }
}

fn replace_calls_in_func(func: &mut LocalFunction, old_id: FunctionId, new_id: FunctionId) {
    let mut stack = vec![func.entry_block()];
    while let Some(seq_id) = stack.pop() {
        let mut builder = func.builder_mut().instr_seq(seq_id);
        for (instr, _) in builder.instrs_mut().iter_mut() {
            match instr {
                Instr::Call(Call { func }) if *func == old_id => {
                    *func = new_id;
                }
                Instr::Block(Block { seq }) | Instr::Loop(Loop { seq }) => {
                    stack.push(*seq);
                }
                Instr::IfElse(IfElse {
                    consequent,
                    alternative,
                }) => {
                    stack.push(*consequent);
                    stack.push(*alternative);
                }
                _ => {}
            }
        }
    }
}
