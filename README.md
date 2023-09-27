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

**Note**: the hashes of private metadata sections are readable by anyone. If a section contains low-entropy data, the attacker could brute-force the contents.
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

Remove unused functions and debug info.

Note: The `icp` metadata sections are preserved through the shrink.

Usage: `ic-wasm <input.wasm> -o <output.wasm> shrink`

### Optimize

Invoke wasm optimizations from [`wasm-opt`](https://github.com/WebAssembly/binaryen).

The optimizer exposes different optimization levels to choose from.

Performance levels (optimizes for runtime):
- O4
- O3 (default setting: best for minimizing cycle usage)
- O2
- O1
- O0 (no optimizations)

Code size levels (optimizes for binary size):
- Oz (best for minimizing code size)
- Os

The recommended setting (O3) reduces cycle usage for Motoko programs by ~10% and  Rust programs by ~4%. The code size for both languages is reduced by ~16%.

Note: The `icp` metadata sections are preserved through the optimizations.

Usage: `ic-wasm <input.wasm> -o <output.wasm> optimize <level>`

There are two further flags exposed from `wasm-opt`:
- `--inline-functions-with-loops`
- `--always-inline-max-function-size <FUNCTION_SIZE>`

These were exposed to aggressively inline functions, which are common in Motoko programs. With the new cost model, there is a large performance gain from inlining functions with loops, but also a large blowup in binary size. Due to the binary size increase, we may not be able to apply this inlining for actor classes inside a Wasm module.

E.g.
`ic-wasm <input.wasm> -o <output.wasm> optimize O3 --inline-functions-with-loops --always-inline-max-function-size 100`

### Resource

Limit resource usage, mainly used by Motoko Playground

Usage: `ic-wasm <input.wasm> -o <output.wasm> resource --remove_cycles_transfer --limit_stable_memory_page 1024`

### Instrument (experimental)

Instrument canister method to emit execution trace to stable memory.

Usage: `ic-wasm <input.wasm> -o <output.wasm> instrument --trace-only func1 --trace-only func2`

Instrumented canister has the following additional endpoints:

* `__get_cycles: () -> (int64) query`. Get the current cycle counter.
* `__get_profiling: () -> (vec { record { int32; int64 }}) query`. Get the execution trace log.
* `__toggle_tracing: () -> ()`. Disable/enable logging the execution trace.
* `__toggle_entry: () -> ()`. Disable/enable clearing exection trace for each update call.
* `icp:public name` metadata. Used to map func_id from execution trace to function name.

When `--trace-only` flag is provided, the counter and trace logging will only happen during the execution of that function, instead of tracing the whole update call. Note that the function itself has to be non-recursive.

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
