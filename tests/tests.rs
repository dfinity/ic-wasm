use assert_cmd::Command;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

fn tests_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
}

/// Path to a fixture file in the `tests` directory.
fn input(wasm: &str) -> PathBuf {
    tests_dir().join(wasm)
}

/// An `ic-wasm` command reading the given input path.
fn ic_wasm(input: impl AsRef<Path>) -> Command {
    // `cargo_bin_cmd!` resolves the built binary via the `CARGO_BIN_EXE_ic-wasm`
    // env var Cargo sets at compile time. The older `Command::cargo_bin` guessed
    // the path from the test executable's location, which breaks under Cargo's
    // custom `build-dir` (rust-lang/cargo#16147, rust-lang/cargo#15010).
    let mut cmd = assert_cmd::cargo_bin_cmd!("ic-wasm");
    // Keep error output deterministic for exact stderr assertions: CI sets
    // `RUST_BACKTRACE=1`, which makes anyhow append a backtrace to stderr.
    cmd.env_remove("RUST_BACKTRACE")
        .env_remove("RUST_LIB_BACKTRACE");
    cmd.arg(input.as_ref());
    cmd
}

/// A fresh, unique temp file to receive `-o` output. Using a distinct file per
/// test lets the suite run concurrently without clobbering shared state.
fn out_wasm() -> NamedTempFile {
    NamedTempFile::new().expect("Failed to create temp output file")
}

/// An `ic-wasm` command reading fixture `wasm` and writing its result to `out`.
fn wasm_input(wasm: &str, out: &NamedTempFile) -> Command {
    let mut cmd = ic_wasm(input(wasm));
    cmd.arg("-o").arg(out.path());
    cmd
}

fn assert_wasm(actual: &Path, expected: &str) {
    let expected = tests_dir().join("ok").join(expected);
    let ok = fs::read(&expected).unwrap();
    let actual_bytes = fs::read(actual).unwrap();
    if ok != actual_bytes {
        use std::env;
        if env::var("REGENERATE_GOLDENFILES").is_ok() {
            let mut f = fs::File::create(&expected).unwrap();
            f.write_all(&actual_bytes).unwrap();
        } else {
            panic!(
                "ic_wasm did not result in expected wasm file: {} != {}. Run \"REGENERATE_GOLDENFILES=1 cargo test\" to update the wasm files",
                expected.display(),
                actual.display()
            );
        }
    }
}

fn assert_functions_are_named(out: &Path) {
    let module = walrus::Module::from_file(out).unwrap();
    let name_count = module.funcs.iter().filter(|f| f.name.is_some()).count();
    let total = module.funcs.iter().count();
    // Walrus doesn't give direct access to the name section, but as a proxy
    // just check that moste functions have names.
    assert!(
        name_count > total / 2,
        "Module has {total} functions but only {name_count} have names."
    )
}

#[test]
fn instrumentation() {
    let out = out_wasm();
    wasm_input("motoko.wasm", &out)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm(out.path(), "motoko-instrument.wasm");
    wasm_input("motoko.wasm", &out)
        .arg("instrument")
        .arg("-t")
        .arg("schedule_copying_gc")
        .assert()
        .success();
    assert_wasm(out.path(), "motoko-gc-instrument.wasm");
    wasm_input("motoko-region.wasm", &out)
        .arg("instrument")
        .arg("-s")
        .arg("16")
        .assert()
        .success();
    assert_wasm(out.path(), "motoko-region-instrument.wasm");
    wasm_input("wat.wasm", &out)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-instrument.wasm");
    wasm_input("wat.wasm.gz", &out)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-instrument.wasm");
    wasm_input("rust.wasm", &out)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm(out.path(), "rust-instrument.wasm");
    wasm_input("rust-region.wasm", &out)
        .arg("instrument")
        .arg("-s")
        .arg("1")
        .assert()
        .success();
    assert_wasm(out.path(), "rust-region-instrument.wasm");
}

#[test]
fn shrink() {
    let out = out_wasm();
    wasm_input("motoko.wasm", &out)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm(out.path(), "motoko-shrink.wasm");
    wasm_input("wat.wasm", &out)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-shrink.wasm");
    wasm_input("wat.wasm.gz", &out)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-shrink.wasm");
    wasm_input("rust.wasm", &out)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm(out.path(), "rust-shrink.wasm");
    wasm_input("classes.wasm", &out)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm(out.path(), "classes-shrink.wasm");
}

#[test]
fn resource() {
    let out = out_wasm();
    wasm_input("motoko.wasm", &out)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm(out.path(), "motoko-limit.wasm");
    wasm_input("wat.wasm", &out)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-limit.wasm");
    wasm_input("wat.wasm.gz", &out)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm(out.path(), "wat-limit.wasm");
    wasm_input("rust.wasm", &out)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm(out.path(), "rust-limit.wasm");
    wasm_input("classes.wasm", &out)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm(out.path(), "classes-limit.wasm");
    let test_canister_id = "zz73r-nyaaa-aabbb-aaaca-cai";
    let management_canister_id = "aaaaa-aa";
    wasm_input("classes.wasm", &out)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(test_canister_id)
        .assert()
        .success();
    assert_wasm(out.path(), "classes-redirect.wasm");
    wasm_input("classes.wasm", &out)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(management_canister_id)
        .assert()
        .success();
    assert_wasm(out.path(), "classes-nop-redirect.wasm");
    wasm_input("evm.wasm", &out)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(test_canister_id)
        .assert()
        .success();
    assert_wasm(out.path(), "evm-redirect.wasm");
}

#[test]
fn info() {
    let expected = r#"Number of types: 6
Number of globals: 1

Number of data sections: 3
Size of data sections: 35 bytes

Number of functions: 9
Number of callbacks: 0
Start function: None
Exported methods: [
    "canister_query get (func_5)",
    "canister_update inc (func_6)",
    "canister_update set (func_7)",
]

Imported IC0 System API: [
    "msg_reply",
    "msg_reply_data_append",
    "msg_arg_data_size",
    "msg_arg_data_copy",
    "trap",
]

Custom sections with size: []
"#;
    ic_wasm(input("wat.wasm"))
        .arg("info")
        .assert()
        .stdout(expected)
        .success();
    ic_wasm(input("wat.wasm.gz"))
        .arg("info")
        .assert()
        .stdout(expected)
        .success();
}

#[test]
#[cfg(feature = "serde")]
fn json_info() {
    let expected = r#"{
  "language": "Unknown",
  "number_of_types": 6,
  "number_of_globals": 1,
  "number_of_data_sections": 3,
  "size_of_data_sections": 35,
  "number_of_functions": 9,
  "number_of_callbacks": 0,
  "start_function": null,
  "exported_methods": [
    {
      "name": "canister_query get",
      "internal_name": "func_5"
    },
    {
      "name": "canister_update inc",
      "internal_name": "func_6"
    },
    {
      "name": "canister_update set",
      "internal_name": "func_7"
    }
  ],
  "imported_ic0_system_api": [
    "msg_reply",
    "msg_reply_data_append",
    "msg_arg_data_size",
    "msg_arg_data_copy",
    "trap"
  ],
  "custom_sections": []
}
"#;
    ic_wasm(input("wat.wasm"))
        .arg("info")
        .arg("--json")
        .assert()
        .stdout(expected)
        .success();
    ic_wasm(input("wat.wasm.gz"))
        .arg("info")
        .arg("--json")
        .assert()
        .stdout(expected)
        .success();
}

#[test]
fn metadata() {
    // List metadata
    ic_wasm(input("motoko.wasm"))
        .arg("metadata")
        .assert()
        .stdout(
            r#"icp:public candid:service
icp:private motoko:stable-types
icp:private motoko:compiler
icp:public candid:args
"#,
        )
        .success();
    // Get motoko:compiler content
    ic_wasm(input("motoko.wasm"))
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.10.0\n")
        .success();
    // Get a non-existed metadata
    ic_wasm(input("motoko.wasm"))
        .arg("metadata")
        .arg("whatever")
        .assert()
        .stdout("Cannot find metadata whatever\n")
        .success();
    // Overwrite motoko:compiler
    let out = out_wasm();
    wasm_input("motoko.wasm", &out)
        .arg("metadata")
        .arg("motoko:compiler")
        .arg("-d")
        .arg("hello")
        .assert()
        .success();
    ic_wasm(out.path())
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("hello\n")
        .success();
    // Add a new metadata
    wasm_input("motoko.wasm", &out)
        .arg("metadata")
        .arg("whatever")
        .arg("-d")
        .arg("what?")
        .arg("-v")
        .arg("public")
        .assert()
        .success();
    ic_wasm(out.path())
        .arg("metadata")
        .assert()
        .stdout(
            r#"icp:public candid:service
icp:private motoko:stable-types
icp:private motoko:compiler
icp:public candid:args
icp:public whatever
"#,
        )
        .success();
}

#[test]
fn metadata_keep_name_section() {
    let out = out_wasm();
    for file in [
        "motoko.wasm",
        "classes.wasm",
        "motoko-region.wasm",
        "rust.wasm",
    ] {
        wasm_input(file, &out)
            .arg("metadata")
            .arg("foo")
            .arg("-d")
            .arg("hello")
            .arg("--keep-name-section")
            .assert()
            .success();
        assert_functions_are_named(out.path());
    }
}

#[test]
#[cfg(feature = "check-endpoints")]
fn check_endpoints() {
    // Candid interface is NOT embedded in wat.wasm
    const CANDID_WITH_MISSING_ENDPOINTS: &str = r#"
    service : () -> {
        inc : (owner: opt principal) -> (nat);
    }
    "#;
    ic_wasm(input("wat.wasm.gz"))
        .arg("check-endpoints")
        .assert()
        .stderr("Error: Candid interface not specified in WASM file and Candid file not provided\n")
        .failure();
    ic_wasm(input("wat.wasm.gz"))
        .arg("check-endpoints")
        .arg("--candid")
        .arg(create_tempfile(CANDID_WITH_MISSING_ENDPOINTS).path())
        .assert()
        .stderr(
            "ERROR: The following endpoint is unexpected in the WASM exports section: canister_update:set\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_query:get\n\
        Error: Canister WASM and Candid interface do not match!\n",
        )
        .failure();
    const HIDDEN_1: &str = r#"
    # Canister update method (this line is a comment)
    canister_update:set
    # Canister query method (this line is also a comment)
    canister_query:get
    "#;
    ic_wasm(input("wat.wasm.gz"))
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_1).path())
        .arg("--candid")
        .arg(create_tempfile(CANDID_WITH_MISSING_ENDPOINTS).path())
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
    // Candid interface is embedded in rust.wasm and motoko.wasm
    ic_wasm(input("rust.wasm"))
        .arg("check-endpoints")
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
    const HIDDEN_2: &str = r#"
    canister_update:dec
    "#;
    ic_wasm(input("rust.wasm"))
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_2).path())
        .assert()
        .stderr("ERROR: The following hidden endpoint is missing from the WASM exports section: canister_update:dec\n\
        Error: Canister WASM and Candid interface do not match!\n")
        .failure();
    ic_wasm(input("motoko.wasm"))
        .arg("check-endpoints")
        .assert()
        .stderr(
        "ERROR: The following endpoint is unexpected in the WASM exports section: canister_update:__motoko_async_helper\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_query:__get_candid_interface_tmp_hack\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_query:__motoko_stable_var_info\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_global_timer\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_init\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_post_upgrade\n\
        ERROR: The following endpoint is unexpected in the WASM exports section: canister_pre_upgrade\n\
        Error: Canister WASM and Candid interface do not match!\n",
        )
        .failure();
    const HIDDEN_3: &str = r#"
    canister_update:__motoko_async_helper
    canister_query:__get_candid_interface_tmp_hack
    canister_query:__motoko_stable_var_info
    canister_global_timer
    canister_init
    canister_post_upgrade
    # The line below is quoted, it is parsed as a JSON string (this line is a comment)
    "canister_pre_upgrade"
    "#;
    ic_wasm(input("motoko.wasm"))
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_3).path())
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
}

#[cfg(feature = "check-endpoints")]
fn create_tempfile(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{content}").expect("Failed to write temp file content");
    temp_file
}

#[test]
fn stub_wasi() {
    let out = out_wasm();

    // First verify input has WASI imports
    let input_module = walrus::Module::from_file(input("wasi-test.wasm")).unwrap();
    let input_wasi_imports: Vec<_> = input_module
        .imports
        .iter()
        .filter(|i| i.module == "wasi_snapshot_preview1")
        .collect();
    assert!(
        !input_wasi_imports.is_empty(),
        "Input should have WASI imports"
    );

    // Test that --stub-wasi removes WASI imports
    wasm_input("wasi-test.wasm", &out)
        .arg("instrument")
        .arg("--stub-wasi")
        .assert()
        .success();

    // Verify the output WASM has no WASI imports
    let module = walrus::Module::from_file(out.path()).unwrap();
    let wasi_imports: Vec<_> = module
        .imports
        .iter()
        .filter(|i| i.module == "wasi_snapshot_preview1")
        .collect();

    assert!(
        wasi_imports.is_empty(),
        "WASI imports should be removed, but found: {:?}",
        wasi_imports.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
}
