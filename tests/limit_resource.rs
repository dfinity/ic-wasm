use ic_wasm::limit_resource::{limit_resource, Config};
use ic_wasm::utils::make_validator_with_features;
use std::collections::HashSet;
use walrus::ir::{dfs_in_order, Value, Visitor};
use walrus::*;
use wasmparser::Validator;

const CYCLES: (i64, i64) = (0, 1_000_000_000_000);

fn filter_config() -> Config {
    Config {
        remove_cycles_add: false,
        filter_cycles_add: true,
        limit_stable_memory_page: None,
        limit_heap_memory_page: None,
        playground_canister_id: None,
    }
}

struct Canister {
    module: Module,
    cycles_add128: FunctionId,
    call_new: FunctionId,
    table: TableId,
}

fn canister() -> Canister {
    let mut module = Module::default();
    module.memories.add_local(false, false, 1, None, None);
    let cycles_add128_ty = module.types.add(&[ValType::I64, ValType::I64], &[]);
    let (cycles_add128, _) = module.add_import_func("ic0", "call_cycles_add128", cycles_add128_ty);
    let call_new_ty = module.types.add(&[ValType::I32; 8], &[]);
    let (call_new, _) = module.add_import_func("ic0", "call_new", call_new_ty);
    let table = module.tables.add_local(false, 1, Some(1), RefType::FUNCREF);
    Canister {
        module,
        cycles_add128,
        call_new,
        table,
    }
}

fn cycles_add128_ty(m: &Module) -> TypeId {
    m.types.find(&[ValType::I64, ValType::I64], &[]).unwrap()
}

fn export_update(m: &mut Module, build: impl FnOnce(&mut InstrSeqBuilder)) {
    let mut builder = FunctionBuilder::new(&mut m.types, &[], &[]);
    build(&mut builder.func_body());
    let func = builder.finish(vec![], &mut m.funcs);
    m.exports.add("canister_update drain", func);
}

fn call_table_slot(body: &mut InstrSeqBuilder, ty: TypeId, table: TableId) {
    body.i64_const(CYCLES.0)
        .i64_const(CYCLES.1)
        .i32_const(0)
        .call_indirect(ty, table);
}

/// Applies the filter and returns every reference to `ic0.<name>` that does not come from
/// one of the wrapper functions `limit_resource` added.
fn unwrapped_refs_after_filter(m: &mut Module, name: &str) -> Vec<String> {
    unwrapped_refs_after_filter_with(m, name, make_validator_with_features())
}

fn unwrapped_refs_after_filter_with(
    m: &mut Module,
    name: &str,
    mut validator: Validator,
) -> Vec<String> {
    let original_funcs: HashSet<_> = m.funcs.iter_local().map(|(id, _)| id).collect();
    limit_resource(m, &filter_config());
    let wasm = m.emit_wasm();
    validator
        .validate_all(&wasm)
        .expect("filtered module should be valid");
    let import = m
        .imports
        .iter()
        .find_map(|i| match i.kind {
            ImportKind::Function(f) if i.module == "ic0" && i.name == name => Some(f),
            _ => None,
        })
        .unwrap();

    let mut refs = vec![];
    for elem in m.elements.iter() {
        let hit = match &elem.items {
            ElementItems::Functions(funcs) => funcs.contains(&import),
            ElementItems::Expressions(_, exprs) => exprs.iter().any(|e| references(e, import)),
        };
        if hit {
            refs.push("element segment".to_string());
        }
    }
    for global in m.globals.iter() {
        if matches!(&global.kind, GlobalKind::Local(init) if references(init, import)) {
            refs.push("global initializer".to_string());
        }
    }
    for table in m.tables.iter() {
        if matches!(&table.init, Some(init) if references(init, import)) {
            refs.push("table initializer".to_string());
        }
    }
    for export in m.exports.iter() {
        if matches!(export.item, ExportItem::Function(f) if f == import) {
            refs.push(format!("export {}", export.name));
        }
    }
    if m.start == Some(import) {
        refs.push("start function".to_string());
    }

    struct FindRefs(FunctionId, usize);
    impl<'i> Visitor<'i> for FindRefs {
        fn visit_function_id(&mut self, func: &FunctionId) {
            self.1 += usize::from(*func == self.0);
        }
    }
    for (id, func) in m.funcs.iter_local() {
        if !original_funcs.contains(&id) {
            continue;
        }
        let mut finder = FindRefs(import, 0);
        dfs_in_order(&mut finder, func, func.entry_block());
        if finder.1 > 0 {
            refs.push(format!("instruction in function {id:?}"));
        }
    }
    refs
}

fn references(expr: &ConstExpr, func: FunctionId) -> bool {
    match expr {
        ConstExpr::RefFunc(f) => *f == func,
        ConstExpr::Extended(ops) => ops
            .iter()
            .any(|op| matches!(op, ConstOp::RefFunc(f) if *f == func)),
        _ => false,
    }
}

#[test]
fn filters_cycles_add_in_active_element_segment() {
    let Canister {
        mut module,
        cycles_add128,
        table,
        ..
    } = canister();
    module.elements.add(
        ElementKind::Active {
            table,
            offset: ConstExpr::Value(Value::I32(0)),
        },
        ElementItems::Functions(vec![cycles_add128]),
    );
    let ty = cycles_add128_ty(&module);
    export_update(&mut module, |body| call_table_slot(body, ty, table));

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_cycles_add_in_element_expressions() {
    let Canister {
        mut module,
        cycles_add128,
        table,
        ..
    } = canister();
    module.elements.add(
        ElementKind::Active {
            table,
            offset: ConstExpr::Value(Value::I32(0)),
        },
        ElementItems::Expressions(RefType::FUNCREF, vec![ConstExpr::RefFunc(cycles_add128)]),
    );
    let ty = cycles_add128_ty(&module);
    export_update(&mut module, |body| call_table_slot(body, ty, table));

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_cycles_add_taken_with_ref_func() {
    let Canister {
        mut module,
        cycles_add128,
        table,
        ..
    } = canister();
    module.elements.add(
        ElementKind::Declared,
        ElementItems::Functions(vec![cycles_add128]),
    );
    let ty = cycles_add128_ty(&module);
    export_update(&mut module, |body| {
        body.i32_const(0).ref_func(cycles_add128).table_set(table);
        call_table_slot(body, ty, table);
    });

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_cycles_add_in_funcref_global() {
    let Canister {
        mut module,
        cycles_add128,
        table,
        ..
    } = canister();
    let global = module.globals.add_local(
        ValType::Ref(RefType::FUNCREF),
        false,
        false,
        ConstExpr::RefFunc(cycles_add128),
    );
    let ty = cycles_add128_ty(&module);
    export_update(&mut module, |body| {
        body.i32_const(0).global_get(global).table_set(table);
        call_table_slot(body, ty, table);
    });

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_cycles_add_in_table_initializer() {
    let Canister {
        mut module,
        cycles_add128,
        ..
    } = canister();
    let table = module.tables.add_local_with_init(
        false,
        1,
        Some(1),
        RefType::FUNCREF,
        Some(ConstExpr::RefFunc(cycles_add128)),
    );
    module.elements.add(
        ElementKind::Declared,
        ElementItems::Functions(vec![cycles_add128]),
    );
    let ty = cycles_add128_ty(&module);
    export_update(&mut module, |body| call_table_slot(body, ty, table));

    // The IC disables function references, which table initializers need.
    assert_eq!(
        unwrapped_refs_after_filter_with(&mut module, "call_cycles_add128", Validator::new()),
        Vec::<String>::new()
    );
}

#[test]
fn filters_cycles_add_reached_with_return_call() {
    let Canister {
        mut module,
        cycles_add128,
        ..
    } = canister();
    let a = module.locals.add(ValType::I64);
    let b = module.locals.add(ValType::I64);
    let mut builder = FunctionBuilder::new(&mut module.types, &[ValType::I64, ValType::I64], &[]);
    builder
        .func_body()
        .local_get(a)
        .local_get(b)
        .return_call(cycles_add128);
    let tail = builder.finish(vec![a, b], &mut module.funcs);
    export_update(&mut module, |body| {
        body.i64_const(CYCLES.0).i64_const(CYCLES.1).call(tail);
    });

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_exported_cycles_add() {
    let Canister {
        mut module,
        cycles_add128,
        ..
    } = canister();
    module.exports.add("cycles_add128", cycles_add128);

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_cycles_add128"),
        Vec::<String>::new()
    );
}

#[test]
fn filters_call_new_in_element_segment() {
    let Canister {
        mut module,
        call_new,
        table,
        ..
    } = canister();
    module.elements.add(
        ElementKind::Active {
            table,
            offset: ConstExpr::Value(Value::I32(0)),
        },
        ElementItems::Functions(vec![call_new]),
    );

    assert_eq!(
        unwrapped_refs_after_filter(&mut module, "call_new"),
        Vec::<String>::new()
    );
}

#[test]
#[should_panic(expected = "should import `ic0.call_cycles_add128` at most once")]
fn rejects_duplicate_cycles_add_import() {
    let Canister { mut module, .. } = canister();
    let ty = cycles_add128_ty(&module);
    let (duplicate, _) = module.add_import_func("ic0", "call_cycles_add128", ty);
    export_update(&mut module, |body| {
        body.i64_const(CYCLES.0).i64_const(CYCLES.1).call(duplicate);
    });

    limit_resource(&mut module, &filter_config());
}
