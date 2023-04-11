use walrus::{IdsToIndices, Module, RawCustomSection};

#[derive(Clone, Copy)]
pub enum Kind {
    Public,
    Private,
}

/// Add or overwrite a metadata section
pub fn add_metadata(m: &mut Module, visibility: Kind, name: &str, data: Vec<u8>) {
    let name = match visibility {
        Kind::Public => "icp:public ".to_owned(),
        Kind::Private => "icp:private ".to_owned(),
    } + name;
    drop(m.customs.remove_raw(&name));
    let custom_section = RawCustomSection { name, data };
    m.customs.add(custom_section);
}

/// Remove a metadata section
pub fn remove_metadata(m: &mut Module, name: &str) {
    let public = "icp:public ".to_owned() + name;
    let private = "icp:private ".to_owned() + name;
    m.customs.remove_raw(&public);
    m.customs.remove_raw(&private);
}

/// List current metadata sections
pub fn list_metadata(m: &Module) -> Vec<&str> {
    m.customs
        .iter()
        .map(|section| section.1.name())
        .filter(|name| name.starts_with("icp:"))
        .collect()
}

/// Get the content of metadata
pub fn get_metadata<'a>(m: &'a Module, name: &'a str) -> Option<std::borrow::Cow<'a, [u8]>> {
    let public = "icp:public ".to_owned() + name;
    let private = "icp:private ".to_owned() + name;
    m.customs
        .iter()
        .find(|(_, section)| section.name() == public || section.name() == private)
        .map(|(_, section)| section.data(&IdsToIndices::default()))
}
