mod candid;

pub use crate::check_endpoints::candid::CandidParser;
use crate::info::ExportedMethodInfo;
use crate::utils::get_exported_methods;
use anyhow::anyhow;
use parse_display::{Display, FromStr};
use std::collections::BTreeSet;
use std::path::PathBuf;
use walrus::Module;

#[derive(Clone, Eq, Debug, Ord, PartialEq, PartialOrd, Display, FromStr)]
pub enum CanisterEndpoint {
    #[display("update:{0}")]
    Update(String),
    #[display("query:{0}")]
    Query(String),
    #[display("composite_query:{0}")]
    CompositeQuery(String),
}

impl TryFrom<&ExportedMethodInfo> for CanisterEndpoint {
    type Error = anyhow::Error;

    fn try_from(method: &ExportedMethodInfo) -> Result<Self, Self::Error> {
        const CANISTER_QUERY_PREFIX: &str = "canister_query ";
        const CANISTER_UPDATE_PREFIX: &str = "canister_update ";
        const CANISTER_COMPOSITE_QUERY_PREFIX: &str = "canister_composite_query ";

        method
            .name
            .strip_prefix(CANISTER_QUERY_PREFIX)
            .map(|q| CanisterEndpoint::Query(q.to_string()))
            .or_else(|| {
                method
                    .name
                    .strip_prefix(CANISTER_UPDATE_PREFIX)
                    .map(|u| CanisterEndpoint::Update(u.to_string()))
            })
            .or_else(|| {
                method
                    .name
                    .strip_prefix(CANISTER_COMPOSITE_QUERY_PREFIX)
                    .map(|u| CanisterEndpoint::CompositeQuery(u.to_string()))
            })
            .ok_or_else(|| {
                anyhow!(
                    "Invalid exported method in canister WASM: '{}'",
                    method.name
                )
            })
    }
}

pub fn check_endpoints(
    module: &Module,
    candid_path: impl Into<PathBuf>,
    hidden_endpoints: &[CanisterEndpoint],
) -> anyhow::Result<()> {
    let wasm_endpoints = get_exported_methods(module)
        .iter()
        .map(CanisterEndpoint::try_from)
        .collect::<Result<BTreeSet<CanisterEndpoint>, _>>()?;

    let candid_endpoints = CandidParser::new(candid_path).parse()?;

    let missing_candid_endpoints = candid_endpoints
        .difference(&wasm_endpoints)
        .collect::<BTreeSet<_>>();
    missing_candid_endpoints.iter().for_each(|endpoint| {
        eprintln!(
            "ERROR: The following Candid endpoint is missing from the WASM exports section: {endpoint}"
        );
    });

    let missing_hidden_endpoints = BTreeSet::from(hidden_endpoints)
        .difference(&wasm_endpoints)
        .collect::<BTreeSet<_>>();
    missing_hidden_endpoints.iter().for_each(|endpoint| {
        eprintln!(
            "ERROR: The following hidden endpoint is missing from the WASM exports section: {endpoint}"
        );
    });

    let unexpected_endpoints = wasm_endpoints
        .iter()
        .filter(|endpoint| {
            !candid_endpoints.contains(endpoint) && !hidden_endpoints.contains(endpoint)
        })
        .collect::<BTreeSet<_>>();
    unexpected_endpoints.iter().for_each(|endpoint| {
        eprintln!(
            "ERROR: The following endpoint is unexpected in the WASM exports section: {endpoint}"
        );
    });

    if !missing_candid_endpoints.is_empty() || !missing_hidden_endpoints.is_empty() || !unexpected_endpoints.is_empty() {
        Err(anyhow!("Canister WASM and Candid interface do not match!"))
    } else {
        println!("Canister WASM and Candid interface match!");
        Ok(())
    }
}
