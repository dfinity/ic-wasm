use crate::utils::{get_func_name, get_motoko_wasm_data_sections, is_motoko_canister};
use walrus::*;

/// Print general summary of the Wasm module
pub fn info(m: &Module) {
    if is_motoko_canister(m) {
        println!("This is a Motoko canister");
        for (_, module) in get_motoko_wasm_data_sections(m) {
            println!("--- Start decoding an embedded Wasm ---");
            info(&module);
            println!("--- End of decoding ---");
        }
        println!();
    }
    println!("Number of types: {}", m.types.iter().count());
    println!("Number of globals: {}", m.globals.iter().count());
    println!();
    let (data, data_size) = m
        .data
        .iter()
        .fold((0, 0), |(count, size), d| (count + 1, size + d.value.len()));
    println!("Number of data sections: {}", data);
    println!("Size of data sections: {} bytes", data_size);
    println!();
    println!("Number of functions: {}", m.funcs.iter().count());
    println!("Number of callbacks: {}", m.elements.iter().count());
    println!(
        "Start function: {:?}",
        m.start.map(|id| get_func_name(m, id))
    );
    let exports: Vec<_> = m
        .exports
        .iter()
        .filter_map(|e| match e.item {
            ExportItem::Function(id) => {
                let name = get_func_name(m, id);
                if e.name == name {
                    Some(e.name.clone())
                } else {
                    Some(format!("{} ({})", e.name, name))
                }
            }
            _ => None,
        })
        .collect();
    println!("Exported methods: {:#?}", exports);
    println!();
    let imports: Vec<&str> = m
        .imports
        .iter()
        .filter(|i| i.module == "ic0")
        .map(|i| i.name.as_ref())
        .collect();
    println!("Imported IC0 System API: {:#?}", imports);
    println!();
    let customs: Vec<_> = m
        .customs
        .iter()
        .map(|(_, s)| format!("{} ({} bytes)", s.name(), s.data(&Default::default()).len()))
        .collect();
    println!("Custom sections with size: {:#?}", customs);
}
