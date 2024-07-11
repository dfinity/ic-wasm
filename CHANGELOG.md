# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

## [0.8.0]

* Upgrade dependency walrus.
  * This enables ic-wasm to process memory64 Wasm modules.

## [0.7.3]

* Enable WebAssembly SIMD in `optimize` subcommand.

## [0.7.2]

* Bump dependency for libflate

## [0.7.1]

* `utils::parse_wasm` and `utils::parse_wasm_file` can take both gzipped and original Wasm inputs.

## [0.3.0 -- 0.7.0]

- Profiling
  + Support profiling stable memory
  + `__get_profiling` supports streaming data download
  + Trace only a subset of functions
  + Add `__toggle_entry` function
  + Use the new cost model for metering
- Add optimize command to use wasm-opt
- Added support for JSON output to `ic-wasm info`.

## [0.2.0] - 2022-09-21

### Changed
- Decoupled library API with walrus (#19)
