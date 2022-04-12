use walrus::*;

/// Print general summary of the Wasm module
pub fn info(m: &Module) {
    println!("Number of types: {}", m.types.iter().count());
    println!("Number of globals: {}", m.globals.iter().count());
    println!();
    let (data, data_size) = m
        .data
        .iter()
        .fold((0, 0), |(count, size), d| (count + 1, size + d.value.len()));
    println!("Number of data sections: {}", data);
    println!("Size of data sections: {}", data_size);
    println!();
    println!("Number of functions: {}", m.funcs.iter().count());
    println!("Number of callbacks: {}", m.elements.iter().count());
    println!(
        "Start function: {:?}",
        m.start.map(|id| crate::utils::get_func_name(m, id))
    );
    let exports: Vec<&str> = m
        .exports
        .iter()
        .filter(|e| matches!(e.item, ExportItem::Function(_)))
        .map(|e| e.name.as_ref())
        .collect();
    println!("Exported methods: {:?}", exports);
    println!();
    let imports: Vec<&str> = m
        .imports
        .iter()
        .filter(|i| i.module == "ic0")
        .map(|i| i.name.as_ref())
        .collect();
    println!("Imported IC0 System API: {:?}", imports);
    println!();
    let customs: Vec<_> = m
        .customs
        .iter()
        .map(|(_, s)| (s.name(), s.data(&Default::default()).len()))
        .collect();
    println!("Custom sections with size: {:?}", customs);
}
