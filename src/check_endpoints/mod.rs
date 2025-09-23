mod candid;

pub use crate::check_endpoints::candid::CandidParser;
use crate::{info::ExportedMethodInfo, utils::get_exported_methods};
use anyhow::anyhow;
use parse_display::{Display, FromStr};
use std::io::{BufRead, BufReader};
use std::str::FromStr;
use std::{collections::BTreeSet, path::Path};
use walrus::Module;

#[derive(Clone, Eq, Debug, Ord, PartialEq, PartialOrd, Display, FromStr)]
pub enum CanisterEndpoint {
    #[display("update:{0}")]
    Update(String),
    #[display("query:{0}")]
    Query(String),
    #[display("composite_query:{0}")]
    CompositeQuery(String),
    #[display("canister_heartbeat")]
    Heartbeat,
    #[display("canister_global_timer")]
    GlobalTimer,
    #[display("canister_init")]
    Init,
    #[display("canister_post_upgrade")]
    PostUpgrade,
    #[display("canister_pre_upgrade")]
    PreUpgrade,
}

impl TryFrom<&ExportedMethodInfo> for CanisterEndpoint {
    type Error = anyhow::Error;

    fn try_from(method: &ExportedMethodInfo) -> Result<Self, Self::Error> {
        let mappings: &[(&str, fn(&str) -> CanisterEndpoint)] = &[
            ("canister_query ", |s| {
                CanisterEndpoint::Query(s.to_string())
            }),
            ("canister_update ", |s| {
                CanisterEndpoint::Update(s.to_string())
            }),
            ("canister_composite_query ", |s| {
                CanisterEndpoint::CompositeQuery(s.to_string())
            }),
            ("canister_heartbeat", |_| CanisterEndpoint::Heartbeat),
            ("canister_global_timer", |_| CanisterEndpoint::GlobalTimer),
            ("canister_init", |_| CanisterEndpoint::Init),
            ("canister_post_upgrade", |_| CanisterEndpoint::PostUpgrade),
            ("canister_pre_upgrade", |_| CanisterEndpoint::PreUpgrade),
        ];

        for (prefix, constructor) in mappings {
            if let Some(rest) = method.name.strip_prefix(prefix) {
                return Ok(constructor(rest));
            }
        }

        Err(anyhow!(
            "Invalid exported method in canister WASM: '{}'",
            method.name
        ))
    }
}

pub fn check_endpoints(
    module: &Module,
    candid_path: Option<&Path>,
    hidden_path: Option<&Path>,
) -> anyhow::Result<()> {
    let wasm_endpoints = get_exported_methods(module)
        .iter()
        .map(CanisterEndpoint::try_from)
        .collect::<Result<BTreeSet<CanisterEndpoint>, _>>()?;

    let candid_endpoints = CandidParser::try_from_wasm(module)?
        .or_else(|| candid_path.map(|path| CandidParser::from_candid_file(&path)))
        .ok_or(anyhow!(
            "Candid interface not specified in WASM file and Candid file not provided"
        ))?
        .parse()?;

    let missing_candid_endpoints = candid_endpoints
        .difference(&wasm_endpoints)
        .collect::<BTreeSet<_>>();
    missing_candid_endpoints.iter().for_each(|endpoint| {
        eprintln!(
            "ERROR: The following Candid endpoint is missing from the WASM exports section: {endpoint}"
        );
    });

    let hidden_endpoints = read_hidden_endpoints(hidden_path)?;
    let missing_hidden_endpoints = hidden_endpoints
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

    if !missing_candid_endpoints.is_empty()
        || !missing_hidden_endpoints.is_empty()
        || !unexpected_endpoints.is_empty()
    {
        Err(anyhow!("Canister WASM and Candid interface do not match!"))
    } else {
        println!("Canister WASM and Candid interface match!");
        Ok(())
    }
}

fn read_hidden_endpoints(maybe_path: Option<&Path>) -> anyhow::Result<BTreeSet<CanisterEndpoint>> {
    if let Some(path) = maybe_path {
        let file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Failed to read hidden endpoints file: {e:?}"))?;
        let reader = BufReader::new(file);
        let lines = reader
            .lines()
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| anyhow!("Failed to read hidden endpoints file: {e:?}"))?;
        let endpoints = lines
            .iter()
            .map(|line| line.trim())
            .filter(|line| !line.is_empty())
            .map(|line| CanisterEndpoint::from_str(line))
            .collect::<Result<BTreeSet<_>, _>>()
            .map_err(|e| anyhow!("Failed to parse hidden endpoints from file: {e:?}"))?;
        Ok(endpoints)
    } else {
        Ok(BTreeSet::new())
    }
}
