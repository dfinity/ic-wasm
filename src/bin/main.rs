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
    Info {
        /// Format the output as JSON
        #[clap(short, long)]
        json: bool,
    },
    /// Remove unused functions and debug info
    Shrink {
        #[clap(short, long)]
        keep_name_section: bool,
    },
    /// Optimize the Wasm module using wasm-opt
    #[cfg(feature = "wasm-opt")]
    Optimize {
        #[clap()]
        level: ic_wasm::optimize::OptLevel,
        #[clap(long("inline-functions-with-loops"))]
        inline_functions_with_loops: bool,
        #[clap(long("always-inline-max-function-size"))]
        always_inline_max_function_size: Option<u32>,
        #[clap(short, long)]
        keep_name_section: bool,
    },
    /// Instrument canister method to emit execution trace to stable memory (experimental)
    Instrument {
        /// Trace only the specified list of functions. The function cannot be recursive
        #[clap(short, long)]
        trace_only: Option<Vec<String>>,
        /// If the canister preallocates a stable memory region, specify the starting page. Required if you want to profile upgrades, or the canister uses stable memory
        #[clap(short, long)]
        start_page: Option<i32>,
        /// The number of pages of the preallocated stable memory
        #[clap(short, long, requires("start_page"))]
        page_limit: Option<i32>,
        /// Use the new metering cost, default to false. This flag will eventually be removed and set to true after the mainnet has adapted the new metering.
        #[clap(short, long)]
        use_new_metering: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    let keep_name_section = match opts.subcommand {
        SubCommand::Shrink { keep_name_section } => keep_name_section,
        #[cfg(feature = "wasm-opt")]
        SubCommand::Optimize {
            keep_name_section, ..
        } => keep_name_section,
        _ => false,
    };
    let mut m = ic_wasm::utils::parse_wasm_file(opts.input, keep_name_section)?;
    match &opts.subcommand {
        SubCommand::Info { json } => {
            let info = ic_wasm::info::WasmInfo::from(&m);
            let mut stdout = std::io::stdout();
            if *json {
                let json = serde_json::to_string_pretty(&info)
                    .expect("Failed to express the Wasm information as JSON.");
                println!("{}", json);
            } else {
                ic_wasm::info::info(&m, &mut stdout)?;
            }
        }
        SubCommand::Shrink { .. } => ic_wasm::shrink::shrink(&mut m),
        #[cfg(feature = "wasm-opt")]
        SubCommand::Optimize {
            level,
            inline_functions_with_loops,
            always_inline_max_function_size,
            ..
        } => ic_wasm::optimize::optimize(
            &mut m,
            level,
            *inline_functions_with_loops,
            always_inline_max_function_size,
            keep_name_section,
        )?,
        SubCommand::Instrument {
            trace_only,
            start_page,
            page_limit,
            use_new_metering,
        } => {
            use ic_wasm::instrumentation::{instrument, Config};
            let config = Config {
                trace_only_funcs: trace_only.clone().unwrap_or(vec![]),
                start_address: start_page.map(|page| page * 65536),
                page_limit: *page_limit,
                use_new_metering: *use_new_metering,
            };
            instrument(&mut m, config).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
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
