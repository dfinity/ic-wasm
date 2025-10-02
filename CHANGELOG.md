# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [unreleased]

* Add GitHub action wrapping the `ic-wasm` CLI.

## [0.9.8] - 2025-10-01

* Fix: `check-endpoints` now correctly handles all exported functions, not just those prefixed with `canister_`.

## [0.9.7] - 2025-09-26

* Add `check-endpoints` command to `ic-wasm`.

## [0.9.6] - 2025-09-17

* Add option to filter cycles transfer.

## [0.9.5] - 2025-01-28

* Fix compilation without default features.

## [0.9.4] - 2025-01-27

* Allow `sign_with_schnorr` in `limit_resource`.

## [0.9.3] - 2025-01-10

* Validate the manipulated module before emitting it and give a warning if that fails.

## [0.9.2] - 2025-01-09

* Fix: limit_resource works with wasm64.

## [0.9.1] - 2024-11-18

* Add redirect for evm canister.

## [0.9.0] - 2024-10-01

* (breaking) Use 64bit API for stable memory in profiling and bump walrus

## [0.8.6] - 2024-09-24

* Add data section check when limiting Wasm heap memory.

## [0.8.5] - 2024-09-05

* Fix http_request redirect.

## [0.8.4] - 2024-09-05

* Add `keep_name_section` option to the `metadata` subcommand.

## [0.8.3] - 2024-08-27

* Fix memory id in limit_resource.

## [0.8.2] - 2024-08-27

* Add support for limiting Wasm heap memory.

## [0.8.1] - 2024-08-20

* Redirect canister snapshot calls in `limit_resource` module.

## [0.8.0] - 2024-07-09

* Upgrade dependency walrus.
  * This enables ic-wasm to process memory64 Wasm modules.

## [0.7.3] - 2024-06-27

* Enable WebAssembly SIMD in `optimize` subcommand.

## [0.7.2] - 2024-04-06

* Bump dependency for libflate

## [0.7.1] - 2024-03-20

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
