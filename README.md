# `ic-wasm`

A library for transforming Wasm canisters running on the Internet Computer

## Executable

To install the `ic-wasm` executable, run

```
$ cargo install ic-wasm
```

### Metadata

Manage metadata in the Wasm module.

Usage: `ic-wasm <input.wasm> [-o <output.wasm>] metadata [name] [-d <text content> | -f <file content>] [-v <public|private>]`

* List current metadata sections
``` 
$ ic-wasm input.wasm metadata
```

* List a specific metadata content
```
$ ic-wasm input.wasm metadata candid:service
```

* Add/overwrite a private metadata section
```
$ ic-wasm input.wasm -o output.wasm metadata new_section -d "hello, world"
```

* Add/overwrite a public metadata section from file
```
$ ic-wasm input.wasm -o output.wasm metadata candid:service -f service.did -v public
```

### Info

Print information about the Wasm canister

Usage: `ic-wasm <input.wasm> info`

### Shrink

Remove unused functions and debug info

Usage: `ic-wasm <input.wasm> -o <output.wasm> shrink`

### Resource

Limit resource usage, mainly used by Motoko Playground

Usage: `ic-wasm <input.wasm> -o <output.wasm> resource --remove_cycles_transfer --limit_stable_memory_page 1024`

### Instrument (experimental)

Instrument canister method to emit execution trace to stable memory. 

Usage: `ic-wasm <input.wasm> -o <output.wasm> instrument`

Current limitations:

* Logs are stored in the first few pages of stable memory (up to 32 pages). This may break:
  + break upgrade
  + break manual access to stable memory
  + `canister_init` in Motoko cannot be profiled, because it uses `stable_size` to decide if there are stable vars to decode
* If heartbeat is present, it's hard to measure any other method calls. It's also hard to measure a specific heartbeat event.
* We only store the first 2M of profiling data.
* We cannot measure query calls.
* No concurrent calls


## Library

To use `ic-wasm` as a library, add this to your `Cargo.toml`:

```toml
[dependencies.ic-wasm]
default-features = false
```

## Contribution

See our [CONTRIBUTING](.github/CONTRIBUTING.md) to get started.
