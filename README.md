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

### Check endpoints

Verify the endpoints a canister’s WASM exports against its Candid interface. This tool is designed to ensure that all exported endpoints are intentional and match the Candid specification, helping to detect any accidental, unexpected or potentially malicious exports.

Usage: `ic-wasm <input.wasm> check-endpoints [--candid <file>] [--hidden <file>]`

- `--candid <file>` (optional) specifies a Candid file containing the canister's expected interface. If omitted, the Candid interface is assumed to be embedded in the WASM file.
- `--hidden <file>` (optional) specifies a file listing endpoints that are intentionally exported by the canister but not present in the Candid interface. Each endpoint should be on a separate line, using one of the following formats:
    - `canister_update:<endpoint name>`
    - `canister_query:<endpoint name>`
    - `canister_composite_query:<endpoint name>`
    - `<endpoint name>`

**Example `hidden.txt`:**
```text
canister_update:__motoko_async_helper
canister_query:__get_candid_interface_tmp_hack
canister_query:__motoko_stable_var_info
canister_global_timer
canister_init
canister_post_upgrade
canister_pre_upgrade
```

### Instrument (experimental)

Instrument canister method to emit execution trace to stable memory.

Usage: `ic-wasm <input.wasm> -o <output.wasm> instrument --trace-only func1 --trace-only func2 --start-page 16 --page-limit 30`

Instrumented canister has the following additional endpoints:

* `__get_cycles: () -> (int64) query`. Get the current cycle counter.
* `__get_profiling: (idx:int32) -> (vec { record { int32; int64 }}, opt int32) query`. Get the execution trace log, starting with `idx` 0. If the log is larger than 2M, it returns the first 2M of trace, and the next `idx` for the next 2M chunk.
* `__toggle_tracing: () -> ()`. Disable/enable logging the execution trace.
* `__toggle_entry: () -> ()`. Disable/enable clearing exection trace for each update call.
* `icp:public name` metadata. Used to map func_id from execution trace to function name.

When `--trace-only` flag is provided, the counter and trace logging will only happen during the execution of that function, instead of tracing the whole update call. Note that the function itself has to be non-recursive.

#### Working with upgrades and stable memory

By default, execution trace is stored in the first few pages (up to 32 pages) of stable memory. Without any user side support, we cannot profile upgrade or code which accesses stable memory. If the canister can pre-allocate a fixed region of stable memory at `canister_init`, we can then pass this address to `ic-wasm` via the `--start-page` flag, so that the trace is written to this pre-allocated space without corrupting the rest of the stable memory access.

Another optional flag `--page-limit` specifies the number of pre-allocated pages in stable memory. By default, it's set to 4096 pages (256MB). We only store trace up to `page-limit` pages, the remaining trace is dropped. 

The recommended way of pre-allocating stable memory is via the `Region` library in Motoko, and `ic-stable-structures` in Rust. But developers are free to use any other libraries or even the raw stable memory system API to pre-allocate space, as long as the developer can guarantee that the pre-allocated space is not touched by the rest of the code.

The following is the code sample for pre-allocating stable memory in Motoko (with `--start-page 16`),

```motoko
import Region "mo:base/Region";
actor {
  stable let profiling = do {
    let r = Region.new();
    ignore Region.grow(r, 4096);  // Increase the page number if you need larger log space
    r;
  };
  ...
}
```

and in Rust (with `--start-page 1`) 

```rust
use ic_stable_structures::{
    memory_manager::{MemoryId, MemoryManager},
    writer::Writer,
    DefaultMemoryImpl, Memory,
};
thread_local! {
    static MEMORY_MANAGER: RefCell<MemoryManager<DefaultMemoryImpl>> =
        RefCell::new(MemoryManager::init(DefaultMemoryImpl::default()));
}
const PROFILING: MemoryId = MemoryId::new(0);
const UPGRADES: MemoryId = MemoryId::new(1);

#[ic_cdk::init]
fn init() {
    let memory = MEMORY_MANAGER.with(|m| m.borrow().get(PROFILING));
    memory.grow(4096);  // Increase the page number if you need larger log space
    ...
}
#[ic_cdk::pre_upgrade]
fn pre_upgrade() {
    let mut memory = MEMORY_MANAGER.with(|m| m.borrow().get(UPGRADES));
    ...
}
#[ic_cdk::post_upgrade]
fn post_upgrade() {
    let memory = MEMORY_MANAGER.with(|m| m.borrow().get(UPGRADES));
    ...
}
```

#### Current limitations

* Without pre-allocating stable memory from user code, we cannot profile upgrade or code that accesses stable memory. You can profile traces larger than 256M, if you pre-allocate large pages of stable memory and specify the `page-limit` flag. Larger traces can be fetched in a streamming fashion via `__get_profiling(idx)`.
* Since the pre-allocation happens in `canister_init`, we cannot profile `canister_init`.
* If heartbeat is present, it's hard to measure any other method calls. It's also hard to measure a specific heartbeat event.
* We cannot measure query calls.
* No concurrent calls.

## Library

To use `ic-wasm` as a library, add this to your `Cargo.toml`:

```toml
[dependencies.ic-wasm]
default-features = false
```

## Contribution

See our [CONTRIBUTING](.github/CONTRIBUTING.md) to get started.
