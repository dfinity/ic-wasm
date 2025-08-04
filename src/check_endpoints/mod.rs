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
}

impl TryFrom<&ExportedMethodInfo> for CanisterEndpoint {
    type Error = anyhow::Error;

    fn try_from(method: &ExportedMethodInfo) -> Result<Self, Self::Error> {
        const CANISTER_QUERY_PREFIX: &str = "canister_query ";
        const CANISTER_UPDATE_PREFIX: &str = "canister_update ";

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
    hidden_endpoints: &Vec<CanisterEndpoint>,
) -> anyhow::Result<()> {
    let wasm_endpoints = get_exported_methods(module)
        .iter()
        .map(|method| CanisterEndpoint::try_from(method))
        .collect::<Result<BTreeSet<CanisterEndpoint>, _>>()?;

    let candid_endpoints = CandidParser::new(candid_path).parse()?;

    let missing_endpoints = candid_endpoints
        .difference(&wasm_endpoints)
        .collect::<BTreeSet<_>>();
    missing_endpoints.iter().for_each(|endpoint| {
        eprintln!(
            "ERROR: The following endpoint is missing from the WASM exports section: {endpoint}"
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

    if !missing_endpoints.is_empty() || !unexpected_endpoints.is_empty() {
        Err(anyhow!("Canister WASM and Candid interface do not match"))
    } else {
        println!("Canister WASM and Candid interface match!");
        Ok(())
    }
}
