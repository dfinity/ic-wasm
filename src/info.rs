use std::io::Write;
use walrus::{ExportItem, Module};

use crate::{utils::*, Error};

pub fn info(wasm: &[u8], output: &mut dyn Write) -> Result<(), Error> {
    let m = walrus::ModuleConfig::new()
        .parse(wasm)
        .map_err(|e| Error::WasmParse(e.to_string()))?;
    info_(&m, output)?;
    Ok(())
}

/// Print general summary of the Wasm module
fn info_(m: &Module, output: &mut dyn Write) -> Result<(), Error> {
    if is_motoko_canister(&m) {
        writeln!(output, "This is a Motoko canister")?;
        for (_, module) in get_motoko_wasm_data_sections(m) {
            writeln!(output, "--- Start decoding an embedded Wasm ---")?;
            info_(&module, output)?;
            writeln!(output, "--- End of decoding ---")?;
        }
        writeln!(output, "")?;
    }
    writeln!(output, "Number of types: {}", m.types.iter().count())?;
    writeln!(output, "Number of globals: {}", m.globals.iter().count())?;
    writeln!(output, "")?;
    let (data, data_size) = m
        .data
        .iter()
        .fold((0, 0), |(count, size), d| (count + 1, size + d.value.len()));
    writeln!(output, "Number of data sections: {}", data)?;
    writeln!(output, "Size of data sections: {} bytes", data_size)?;
    writeln!(output, "")?;
    writeln!(output, "Number of functions: {}", m.funcs.iter().count())?;
    writeln!(output, "Number of callbacks: {}", m.elements.iter().count())?;
    writeln!(
        output,
        "Start function: {:?}",
        m.start.map(|id| get_func_name(m, id))
    )?;
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
    writeln!(output, "Exported methods: {:#?}", exports)?;
    writeln!(output, "")?;
    let imports: Vec<&str> = m
        .imports
        .iter()
        .filter(|i| i.module == "ic0")
        .map(|i| i.name.as_ref())
        .collect();
    writeln!(output, "Imported IC0 System API: {:#?}", imports)?;
    writeln!(output, "")?;
    let customs: Vec<_> = m
        .customs
        .iter()
        .map(|(_, s)| format!("{} ({} bytes)", s.name(), s.data(&Default::default()).len()))
        .collect();
    writeln!(output, "Custom sections with size: {:#?}", customs)?;
    Ok(())
}
