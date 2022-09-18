use walrus::{Module, RawCustomSection, IdsToIndices};

use crate::Error;

pub enum Kind {
    Public,
    Private,
}

fn wasm_to_module(wasm: &[u8]) -> Result<Module, Error> {
    walrus::ModuleConfig::new()
        .parse(wasm)
        .map_err(|e| Error::WasmParse(e.to_string()))
}

/// Add or overwrite a metadata section
pub fn add_metadata(
    wasm: &[u8],
    visibility: Kind,
    name: &str,
    data: Vec<u8>,
) -> Result<Vec<u8>, Error> {
    let mut m = wasm_to_module(wasm)?;
    let name = match visibility {
        Kind::Public => "icp:public ".to_owned(),
        Kind::Private => "icp:private ".to_owned(),
    } + name;
    drop(m.customs.remove_raw(&name));
    let custom_section = RawCustomSection { name, data };
    m.customs.add(custom_section);
    Ok(m.emit_wasm())
}

/// Remove a metadata section
pub fn remove_metadata(wasm: &[u8], name: &str) -> Result<Vec<u8>, Error> {
    let mut m = wasm_to_module(wasm)?;
    let public = "icp:public ".to_owned() + name;
    let private = "icp:private ".to_owned() + name;
    m.customs.remove_raw(&public);
    m.customs.remove_raw(&private);
    Ok(m.emit_wasm())
}

/// List current metadata sections
pub fn list_metadata(wasm: &[u8]) -> Result<Vec<String>, Error> {
    let m = wasm_to_module(wasm)?;
    Ok(m.customs
        .iter()
        .map(|section| section.1.name().to_string())
        .filter(|name| name.starts_with("icp:"))
        .collect())
}

/// Get the content of metadata
pub fn get_metadata<'a>(wasm: &[u8], name: &'a str) -> Result<String, Error> {
    let m = wasm_to_module(wasm)?;
    let public = "icp:public ".to_owned() + name;
    let private = "icp:private ".to_owned() + name;
    let r = m
        .customs
        .iter()
        .find(|(_, section)| section.name() == public || section.name() == private)
        .map(|(_, section)| section.data(&IdsToIndices::default()).clone());
    match r {
        Some(data) => {
            let s = String::from_utf8_lossy(&data).into_owned();
            Ok(s)
        }
        None => Err(Error::MetadataNotFound(name.to_string())),
    }
}
