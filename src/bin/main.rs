use clap::{crate_authors, crate_version, Parser};
use std::path::PathBuf;

#[derive(Parser)]
#[clap(
version = crate_version!(),
author = crate_authors!(),
)]
struct Opts {
    /// Input Wasm file.
    input: PathBuf,

    /// Write the transformed Wasm file if provided.
    #[clap(short, long)]
    output: Option<PathBuf>,

    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    /// Manage metadata in the Wasm module
    Metadata {
        /// Name of metadata. If not provided, list the current metadata sections.
        name: Option<String>,
        /// Content of metadata as a string
        #[clap(short, long, requires("name"))]
        data: Option<String>,
        /// Content of metadata from a file
        #[clap(short, long, requires("name"), conflicts_with("data"))]
        file: Option<PathBuf>,
        /// Visibility of metadata
        #[clap(short, long, value_parser = ["public", "private"], default_value = "private")]
        visibility: String,
    },
    /// Limit resource usage
    Resource {
        /// Remove cycles_add system API call
        #[clap(short, long)]
        remove_cycles_transfer: bool,
        /// Allocate at most specified amount of memory pages for stable memory
        #[clap(short, long)]
        limit_stable_memory_page: Option<u32>,
        /// Redirects controller system API calls to specified motoko backend canister ID
        #[clap(short, long)]
        playground_backend_redirect: Option<candid::Principal>,
    },
    /// List information about the Wasm canister
    Info,
    /// Remove unused functions and debug info. Optionally, optimize the Wasm code
    Shrink {
        #[clap(short, long, value_parser = ["O0", "O1", "O2", "O3", "O4", "Os", "Oz"])]
        optimize: Option<String>,
    },
    /// Instrument canister method to emit execution trace to stable memory (experimental)
    Instrument {
        #[clap(short, long)]
        trace_only: Option<Vec<String>>,
    },
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    let keep_name_section = matches!(opts.subcommand, SubCommand::Shrink { .. });
    let mut m = ic_wasm::utils::parse_wasm_file(opts.input, keep_name_section)?;
    match &opts.subcommand {
        SubCommand::Info => {
            let mut stdout = std::io::stdout();
            ic_wasm::info::info(&m, &mut stdout)?;
        }
        SubCommand::Shrink { optimize } => {
            use ic_wasm::shrink;
            match optimize {
                Some(level) => shrink::shrink_with_wasm_opt(&mut m, level)?,
                None => shrink::shrink(&mut m),
            }
        }
        SubCommand::Instrument { trace_only } => match trace_only {
            None => ic_wasm::instrumentation::instrument(&mut m, &[]),
            Some(vec) => ic_wasm::instrumentation::instrument(&mut m, vec),
        }
        .map_err(|e| anyhow::anyhow!("{e}"))?,
        SubCommand::Resource {
            remove_cycles_transfer,
            limit_stable_memory_page,
            playground_backend_redirect,
        } => {
            use ic_wasm::limit_resource::{limit_resource, Config};
            let config = Config {
                remove_cycles_add: *remove_cycles_transfer,
                limit_stable_memory_page: *limit_stable_memory_page,
                playground_canister_id: *playground_backend_redirect,
            };
            limit_resource(&mut m, &config)
        }
        SubCommand::Metadata {
            name,
            data,
            file,
            visibility,
        } => {
            use ic_wasm::metadata::*;
            if let Some(name) = name {
                let visibility = match visibility.as_str() {
                    "public" => Kind::Public,
                    "private" => Kind::Private,
                    _ => unreachable!(),
                };
                let data = match (data, file) {
                    (Some(data), None) => data.as_bytes().to_vec(),
                    (None, Some(path)) => std::fs::read(path)?,
                    (None, None) => {
                        let data = get_metadata(&m, name);
                        if let Some(data) = data {
                            println!("{}", String::from_utf8_lossy(&data));
                        } else {
                            println!("Cannot find metadata {name}");
                        }
                        return Ok(());
                    }
                    (_, _) => unreachable!(),
                };
                add_metadata(&mut m, visibility, name, data)
            } else {
                let names = list_metadata(&m);
                for name in names.iter() {
                    println!("{name}");
                }
                return Ok(());
            }
        }
    };
    if let Some(output) = opts.output {
        m.emit_wasm_file(output)?;
    }
    Ok(())
}
