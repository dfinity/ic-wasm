use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::Write;
use walrus::{ExportItem, Module};

use crate::{utils::*, Error};

/// External information about a Wasm, such as API methods.
#[derive(Serialize, Deserialize)]
pub struct WasmInfo {
    language: LanguageSpecificInfo,
    number_of_types: usize,
    number_of_globals: usize,
    number_of_data_sections: usize,
    size_of_data_sections: usize,
}

/// External information that is specific to one language
#[derive(Serialize, Deserialize)]
pub enum LanguageSpecificInfo {
    Motoko {
        embedded_wasm: Vec<(String, WasmInfo)>,
    },
    Unknown,
}

impl From<&Module> for WasmInfo {
    fn from(m: &Module) -> WasmInfo {
        let (number_of_data_sections, size_of_data_sections) = m
            .data
            .iter()
            .fold((0, 0), |(count, size), d| (count + 1, size + d.value.len()));
        WasmInfo {
            language: LanguageSpecificInfo::from(m),
            number_of_types: m.types.iter().count(),
            number_of_globals: m.globals.iter().count(),
            number_of_data_sections,
            size_of_data_sections,
        }
    }
}

impl From<&Module> for LanguageSpecificInfo {
    fn from(m: &Module) -> LanguageSpecificInfo {
        if is_motoko_canister(m) {
            let mut embedded_wasm = Vec::new();
            for (data_id, embedded_module) in get_motoko_wasm_data_sections(m) {
                embedded_wasm.push((format!("{:?}", data_id), WasmInfo::from(&embedded_module)));
            }
            return LanguageSpecificInfo::Motoko { embedded_wasm };
        }
        LanguageSpecificInfo::Unknown
    }
}

impl fmt::Display for WasmInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.language)?;
        writeln!(f, "Number of types: {}", self.number_of_types)?;
        writeln!(f, "Number of globals: {}", self.number_of_globals)?;
        writeln!(
            f,
            "Number of data sections: {}",
            self.number_of_data_sections
        )?;
        writeln!(
            f,
            "Size of data sections: {} bytes",
            self.size_of_data_sections
        )?;
        writeln!(f)
    }
}

impl fmt::Display for LanguageSpecificInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LanguageSpecificInfo::Motoko { embedded_wasm } => {
                writeln!(f, "This is a Motoko canister")?;
                for (_, wasm_info) in embedded_wasm {
                    writeln!(f, "--- Start decoding an embedded Wasm ---")?;
                    write!(f, "{}", wasm_info)?;
                    writeln!(f, "--- End of decoding ---")?;
                }
                writeln!(f)
            }
            LanguageSpecificInfo::Unknown => Ok(()),
        }
    }
}

/// Print general summary of the Wasm module
pub fn info(m: &Module, output: &mut dyn Write) -> Result<(), Error> {
    if is_motoko_canister(m) {
        writeln!(output, "This is a Motoko canister")?;
        for (_, module) in get_motoko_wasm_data_sections(m) {
            writeln!(output, "--- Start decoding an embedded Wasm ---")?;
            info(&module, output)?;
            writeln!(output, "--- End of decoding ---")?;
        }
        writeln!(output)?;
    }
    writeln!(output, "Number of types: {}", m.types.iter().count())?;
    writeln!(output, "Number of globals: {}", m.globals.iter().count())?;
    writeln!(output)?;
    let (data, data_size) = m
        .data
        .iter()
        .fold((0, 0), |(count, size), d| (count + 1, size + d.value.len()));
    writeln!(output, "Number of data sections: {data}")?;
    writeln!(output, "Size of data sections: {data_size} bytes")?;
    writeln!(output)?;
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
    writeln!(output, "Exported methods: {exports:#?}")?;
    writeln!(output)?;
    let imports: Vec<&str> = m
        .imports
        .iter()
        .filter(|i| i.module == "ic0")
        .map(|i| i.name.as_ref())
        .collect();
    writeln!(output, "Imported IC0 System API: {imports:#?}")?;
    writeln!(output)?;
    let customs: Vec<_> = m
        .customs
        .iter()
        .map(|(_, s)| format!("{} ({} bytes)", s.name(), s.data(&Default::default()).len()))
        .collect();
    writeln!(output, "Custom sections with size: {customs:#?}")?;
    Ok(())
}
