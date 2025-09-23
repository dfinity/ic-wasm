use crate::check_endpoints::CanisterEndpoint;
use anyhow::{format_err, Error, Result};
use candid::types::{FuncMode, Function, TypeInner};
use candid_parser::utils::CandidSource;
use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::Path;
use std::str;
use walrus::{IdsToIndices, Module};

pub struct CandidParser<'a> {
    source: CandidSource<'a>,
}

impl<'a> From<CandidSource<'a>> for CandidParser<'a> {
    fn from(source: CandidSource<'a>) -> Self {
        Self { source }
    }
}

impl<'a> CandidParser<'a> {
    pub fn from_candid_file(path: &'a Path) -> Self {
        Self::from(CandidSource::File(path))
    }

    pub fn try_from_wasm(module: &'a Module) -> Result<Option<Self>> {
        module
            .customs
            .iter()
            .filter(|(_, s)| s.name() == "icp:public candid:service")
            .next()
            .map(|(_, s)| {
                let bytes = match s.data(&IdsToIndices::default()) {
                    Cow::Borrowed(bytes) => bytes,
                    Cow::Owned(_) => unreachable!(),
                };
                let candid = str::from_utf8(&bytes).map_err(|e| {
                    format_err!("Cannot interpret WASM custom section as text: {e:?}")
                })?;
                Ok(Self::from(CandidSource::Text(candid)))
            })
            .transpose()
    }
}

impl CandidParser<'_> {
    pub fn parse(&self) -> Result<BTreeSet<CanisterEndpoint>> {
        let (_, top_level) = self.source.load()?;

        let maybe_actor = match top_level {
            Some(actor) => actor,
            None => return Err(Error::msg("Top-level definition not found")),
        };

        let service = match maybe_actor.as_ref() {
            TypeInner::Class(_, class) => class,
            service => service,
        };

        let functions = match service {
            TypeInner::Service(functions) => functions,
            _ => return Err(Error::msg("Top-level service definition not found")),
        };

        let endpoints = functions
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

        Ok(endpoints)
    }
}
