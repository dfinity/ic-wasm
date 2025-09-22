use crate::check_endpoints::CanisterEndpoint;
use anyhow::{format_err, Error, Result};
use candid::types::{FuncMode, Function, TypeInner};
use candid_parser::utils::CandidSource;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::str;
use walrus::{IdsToIndices, Module};

pub struct CandidParser<'a> {
    source: CandidSource<'a>,
}

impl From<CandidSource<'_>> for CandidParser<'_> {
    fn from(source: CandidSource) -> Self {
        Self { source }
    }
}

impl CandidParser<'_> {
    pub fn from_candid_file(path: impl Into<PathBuf>) -> Self {
        Self::from(CandidSource::File(&path.into()))
    }

    pub fn try_from_wasm(module: &Module) -> Result<Option<Self>> {
        module
            .customs
            .iter()
            .filter(|(_, s)| s.name() == "icp:public candid:service")
            .next()
            .map(|(_, s)| {
                let candid = str::from_utf8(&s.data(&IdsToIndices::default())).map_err(|e| {
                    format_err!("Cannot interpret WASM custom section as text: {e:?}")
                })?;
                Ok(Self::from(CandidSource::Text(candid)))
            })
            .transpose()
    }

    pub fn parse(&self) -> Result<BTreeSet<CanisterEndpoint>> {
        let (_, maybe_actor) = self.source.load()?;

        let maybe_class =
            maybe_actor.ok_or_else(|| Error::msg("Top-level actor definition not found"))?;

        let maybe_service = match maybe_class.as_ref() {
            TypeInner::Class(_, class) => class,
            _ => return Err(Error::msg("Top-level class definition not found")),
        };

        let maybe_functions = match maybe_service.as_ref() {
            TypeInner::Service(maybe_functions) => maybe_functions,
            _ => return Err(Error::msg("Top-level service definition not found")),
        };

        let functions = maybe_functions
            .iter()
            .filter_map(|(name, maybe_function)| {
                if let TypeInner::Func(Function { modes, .. }) = maybe_function.as_ref() {
                    if modes.contains(&FuncMode::Query) || modes.contains(&FuncMode::CompositeQuery)
                    {
                        Some(CanisterEndpoint::Query(name.to_string()))
                    } else {
                        Some(CanisterEndpoint::Update(name.to_string()))
                    }
                } else {
                    None
                }
            })
            .collect();

        Ok(functions)
    }
}
