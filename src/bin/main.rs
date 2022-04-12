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
        #[clap(short, long, possible_values = &["public", "private"], default_value = "private")]
        visibility: String,
    },
    /// List information about the Wasm canister
    Info,
    /// Remove unused functions and debug info
    Shrink,
    /// Instrument canister method to emit execution trace to stable memory (experimental)
    Instrument,
}

fn walrus_config_from_options(opts: &Opts) -> walrus::ModuleConfig {
    let mut config = walrus::ModuleConfig::new();
    if let SubCommand::Shrink = opts.subcommand {
        config.generate_name_section(false);
        config.generate_producers_section(false);
    }
    config
}

fn main() -> anyhow::Result<()> {
    let opts: Opts = Opts::parse();
    let config = walrus_config_from_options(&opts);
    let mut m = config.parse_file(opts.input)?;
    match &opts.subcommand {
        SubCommand::Info => {
            ic_wasm::info::info(&m);
        }
        SubCommand::Shrink => {
            ic_wasm::shrink::shrink(&mut m);
        }
        SubCommand::Instrument => {
            ic_wasm::instrumentation::instrument(&mut m);
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
                    (None, Some(path)) => std::fs::read(&path)?,
                    (None, None) => {
                        let data = get_metadata(&m, name);
                        if let Some(data) = data {
                            println!("{}", String::from_utf8_lossy(&data));
                        } else {
                            println!("Cannot find metadata {}", name);
                        }
                        return Ok(());
                    }
                    (_, _) => unreachable!(),
                };
                add_metadata(&mut m, visibility, name, data);
            } else {
                let names = list_metadata(&m);
                for name in names.iter() {
                    println!("{}", name);
                }
                return Ok(());
            }
        }
    }
    if let Some(output) = opts.output {
        m.emit_wasm_file(output)?;
    }
    Ok(())
}
