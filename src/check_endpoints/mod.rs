mod candid;

pub use crate::check_endpoints::candid::CandidParser;
use crate::{info::ExportedMethodInfo, utils::get_exported_methods};
use anyhow::anyhow;
use parse_display::{Display, FromStr};
use std::io::BufReader;
use std::{collections::BTreeSet, io::BufRead, path::Path, str::FromStr};
use walrus::Module;

#[derive(Clone, Eq, Debug, Ord, PartialEq, PartialOrd, Display, FromStr)]
pub enum CanisterEndpoint {
    #[display("canister_update:{0}")]
    Update(String),
    #[display("canister_query:{0}")]
    Query(String),
    #[display("canister_composite_query:{0}")]
    CompositeQuery(String),
    #[display("{0}")]
    Entrypoint(String),
}

impl TryFrom<&ExportedMethodInfo> for CanisterEndpoint {
    type Error = anyhow::Error;

    fn try_from(method: &ExportedMethodInfo) -> Result<Self, Self::Error> {
        type EndpointConstructor = fn(&str) -> CanisterEndpoint;
        const MAPPINGS: &[(&str, EndpointConstructor)] = &[
            ("canister_update", |s| {
                CanisterEndpoint::Update(s.to_string())
            }),
            ("canister_query", |s| CanisterEndpoint::Query(s.to_string())),
            ("canister_composite_query", |s| {
                CanisterEndpoint::CompositeQuery(s.to_string())
            }),
        ];

        for (candid_prefix, constructor) in MAPPINGS {
            if let Some(rest) = method.name.strip_prefix(candid_prefix) {
                return Ok(constructor(rest.trim()));
            }
        }

        let trimmed = method.name.trim();
        if !trimmed.is_empty() {
            Ok(CanisterEndpoint::Entrypoint(trimmed.to_string()))
        } else {
            Err(anyhow!("Exported method in canister WASM has empty name"))
        }
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
        .or_else(|| candid_path.map(CandidParser::from_candid_file))
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
        let mut endpoints = BTreeSet::new();
        for line in read_lines(path)? {
            if let Some(endpoint) = parse_line(line)? {
                endpoints.insert(endpoint);
            }
        }
        Ok(endpoints)
    } else {
        Ok(BTreeSet::new())
    }
}

fn read_lines(path: &Path) -> anyhow::Result<Vec<String>> {
    let file = std::fs::File::open(path)
        .map_err(|e| anyhow!("Could not open hidden endpoints file: {e:?}"))?;

    let reader = BufReader::new(file);
    let mut lines = Vec::new();

    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            lines.push(trimmed.to_string());
        }
    }

    Ok(lines)
}

fn parse_line(line: String) -> anyhow::Result<Option<CanisterEndpoint>> {
    fn parse_uncommented_line(line: String) -> anyhow::Result<Option<CanisterEndpoint>> {
        CanisterEndpoint::from_str(line.as_str())
            .map(Some)
            .map_err(Into::into)
    }
    // Comment: ignore line
    if line.starts_with("#") {
        return Ok(None);
    }
    // Quoted line: use JSON string syntax to allow for character escaping
    if line.starts_with('"') {
        if !line.ends_with('"') || line.len() < 2 {
            return Err(anyhow!(
                "Could not parse hidden endpoint, missing terminating quote: {line}"
            ));
        }
        return serde_json::from_str::<String>(&line)
            .map_err(|e| anyhow!("Could not parse hidden endpoint: {e:?}"))
            .and_then(parse_uncommented_line);
    }
    // Regular line
    parse_uncommented_line(line)
}
