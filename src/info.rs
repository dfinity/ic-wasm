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
    number_of_functions: usize,
    number_of_callbacks: usize,
    start_function: Option<String>,
    exported_methods: Vec<ExportedMethodInfo>,
    imported_ic0_system_api: Vec<String>,
    custom_sections: Vec<CustomSectionInfo>,
}

/// External information that is specific to one language
#[derive(Serialize, Deserialize)]
pub enum LanguageSpecificInfo {
    Motoko {
        embedded_wasm: Vec<(String, WasmInfo)>,
    },
    Unknown,
}

/// Information about an exported method.
#[derive(Serialize, Deserialize)]
pub struct ExportedMethodInfo {
    name: String,
    internal_name: String,
}

/// Statistics about a custom section.
#[derive(Serialize, Deserialize)]
pub struct CustomSectionInfo {
    name: String,
    size: usize,
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
            number_of_functions: m.funcs.iter().count(),
            number_of_callbacks: m.elements.iter().count(),
            start_function: m.start.map(|id| get_func_name(m, id)),
            exported_methods: m
                .exports
                .iter()
                .filter_map(|e| match e.item {
                    ExportItem::Function(id) => Some(ExportedMethodInfo {
                        name: e.name.clone(),
                        internal_name: get_func_name(m, id),
                    }),
                    _ => None,
                })
                .collect(),
            imported_ic0_system_api: m
                .imports
                .iter()
                .filter(|i| i.module == "ic0")
                .map(|i| i.name.clone())
                .collect(),
            custom_sections: m
                .customs
                .iter()
                .map(|(_, s)| CustomSectionInfo {
                    name: s.name().to_string(),
                    size: s.data(&Default::default()).len(),
                })
                .collect(),
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
        writeln!(f)?;
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
        writeln!(f)?;
        writeln!(f, "Number of functions: {}", self.number_of_functions)?;
        writeln!(f, "Number of callbacks: {}", self.number_of_callbacks)?;
        writeln!(f, "Start function: {:?}", self.start_function)?;
        let exports: Vec<_> = self
            .exported_methods
            .iter()
            .map(
                |ExportedMethodInfo {
                     name,
                     internal_name,
                 }| {
                    if name == internal_name {
                        internal_name.clone()
                    } else {
                        format!("{name} ({internal_name})")
                    }
                },
            )
            .collect();
        writeln!(f, "Exported methods: {exports:#?}")?;
        writeln!(f)?;
        writeln!(
            f,
            "Imported IC0 System API: {:#?}",
            self.imported_ic0_system_api
        )?;
        writeln!(f)?;
        let customs: Vec<_> = self
            .custom_sections
            .iter()
            .map(|section_info| format!("{} ({} bytes)", section_info.name, section_info.size))
            .collect();
        writeln!(f, "Custom sections with size: {customs:#?}")?;
        Ok(())
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
    write!(output, "{}", WasmInfo::from(m))?;
    Ok(())
}
