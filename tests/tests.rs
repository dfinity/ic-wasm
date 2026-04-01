use assert_cmd::Command;

use std::fs;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;

fn wasm_input(wasm: &str, output: bool) -> Command {
    let mut cmd = Command::cargo_bin("ic-wasm").unwrap();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    cmd.arg(path.join(wasm));
    if output {
        cmd.arg("-o").arg(path.join("out.wasm"));
    }
    cmd
}

fn assert_wasm(expected: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let expected = path.join("ok").join(expected);
    let out = path.join("out.wasm");
    let ok = fs::read(&expected).unwrap();
    let actual = fs::read(&out).unwrap();
    if ok != actual {
        use std::env;
        use std::io::Write;
        if env::var("REGENERATE_GOLDENFILES").is_ok() {
            let mut f = fs::File::create(&expected).unwrap();
            f.write_all(&actual).unwrap();
        } else {
            panic!(
                "ic_wasm did not result in expected wasm file: {} != {}. Run \"REGENERATE_GOLDENFILES=1 cargo test\" to update the wasm files",
                expected.display(),
                out.display()
            );
        }
    }
}

fn assert_functions_are_named() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let out = path.join("out.wasm");

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
    wasm_input("motoko.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("motoko-instrument.wasm");
    wasm_input("motoko.wasm", true)
        .arg("instrument")
        .arg("-t")
        .arg("schedule_copying_gc")
        .assert()
        .success();
    assert_wasm("motoko-gc-instrument.wasm");
    wasm_input("motoko-region.wasm", true)
        .arg("instrument")
        .arg("-s")
        .arg("16")
        .assert()
        .success();
    assert_wasm("motoko-region-instrument.wasm");
    wasm_input("wat.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("wat-instrument.wasm");
    wasm_input("wat.wasm.gz", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("wat-instrument.wasm");
    wasm_input("rust.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("rust-instrument.wasm");
    wasm_input("rust-region.wasm", true)
        .arg("instrument")
        .arg("-s")
        .arg("1")
        .assert()
        .success();
    assert_wasm("rust-region-instrument.wasm");
}

#[test]
fn shrink() {
    wasm_input("motoko.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("motoko-shrink.wasm");
    wasm_input("wat.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("wat-shrink.wasm");
    wasm_input("wat.wasm.gz", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("wat-shrink.wasm");
    wasm_input("rust.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("rust-shrink.wasm");
    wasm_input("classes.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("classes-shrink.wasm");
}
#[test]
fn optimize() {
    let expected_metadata = r#"icp:public candid:service
icp:private candid:args
icp:private motoko:stable-types
icp:private motoko:compiler
"#;

    wasm_input("classes.wasm", true)
        .arg("optimize")
        .arg("O3")
        .arg("--inline-functions-with-loops")
        .arg("--always-inline-max-function-size")
        .arg("100")
        .assert()
        .success();
    assert_wasm("classes-optimize.wasm");
    wasm_input("ok/classes-optimize.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("classes.wasm", true)
        .arg("optimize")
        .arg("O3")
        .arg("--keep-name-section")
        .assert()
        .success();
    assert_wasm("classes-optimize-names.wasm");
}

#[test]
fn resource() {
    wasm_input("motoko.wasm", true)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm("motoko-limit.wasm");
    wasm_input("wat.wasm", true)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm("wat-limit.wasm");
    wasm_input("wat.wasm.gz", true)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm("wat-limit.wasm");
    wasm_input("rust.wasm", true)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm("rust-limit.wasm");
    wasm_input("classes.wasm", true)
        .arg("resource")
        .arg("--remove-cycles-transfer")
        .arg("--limit-stable-memory-page")
        .arg("32")
        .assert()
        .success();
    assert_wasm("classes-limit.wasm");
    let test_canister_id = "zz73r-nyaaa-aabbb-aaaca-cai";
    let management_canister_id = "aaaaa-aa";
    wasm_input("classes.wasm", true)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(test_canister_id)
        .assert()
        .success();
    assert_wasm("classes-redirect.wasm");
    wasm_input("classes.wasm", true)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(management_canister_id)
        .assert()
        .success();
    assert_wasm("classes-nop-redirect.wasm");
    wasm_input("evm.wasm", true)
        .arg("resource")
        .arg("--playground-backend-redirect")
        .arg(test_canister_id)
        .assert()
        .success();
    assert_wasm("evm-redirect.wasm");
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
    wasm_input("wat.wasm", false)
        .arg("info")
        .assert()
        .stdout(expected)
        .success();
    wasm_input("wat.wasm.gz", false)
        .arg("info")
        .assert()
        .stdout(expected)
        .success();
}

#[test]
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
    wasm_input("wat.wasm", false)
        .arg("info")
        .arg("--json")
        .assert()
        .stdout(expected)
        .success();
    wasm_input("wat.wasm.gz", false)
        .arg("info")
        .arg("--json")
        .assert()
        .stdout(expected)
        .success();
}

#[test]
fn metadata() {
    // List metadata
    wasm_input("motoko.wasm", false)
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
    wasm_input("motoko.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.10.0\n")
        .success();
    // Get a non-existed metadata
    wasm_input("motoko.wasm", false)
        .arg("metadata")
        .arg("whatever")
        .assert()
        .stdout("Cannot find metadata whatever\n")
        .success();
    // Overwrite motoko:compiler
    wasm_input("motoko.wasm", true)
        .arg("metadata")
        .arg("motoko:compiler")
        .arg("-d")
        .arg("hello")
        .assert()
        .success();
    wasm_input("out.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("hello\n")
        .success();
    // Add a new metadata
    wasm_input("motoko.wasm", true)
        .arg("metadata")
        .arg("whatever")
        .arg("-d")
        .arg("what?")
        .arg("-v")
        .arg("public")
        .assert()
        .success();
    wasm_input("out.wasm", false)
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
    for file in [
        "motoko.wasm",
        "classes.wasm",
        "motoko-region.wasm",
        "rust.wasm",
    ] {
        wasm_input(file, true)
            .arg("metadata")
            .arg("foo")
            .arg("-d")
            .arg("hello")
            .arg("--keep-name-section")
            .assert()
            .success();
        assert_functions_are_named();
    }
}

#[test]
fn check_endpoints() {
    // Candid interface is NOT embedded in wat.wasm
    const CANDID_WITH_MISSING_ENDPOINTS: &str = r#"
    service : () -> {
        inc : (owner: opt principal) -> (nat);
    }
    "#;
    wasm_input("wat.wasm.gz", false)
        .arg("check-endpoints")
        .assert()
        .stderr("Error: Candid interface not specified in WASM file and Candid file not provided\n")
        .failure();
    wasm_input("wat.wasm.gz", false)
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
    wasm_input("wat.wasm.gz", false)
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_1).path())
        .arg("--candid")
        .arg(create_tempfile(CANDID_WITH_MISSING_ENDPOINTS).path())
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
    // Candid interface is embedded in rust.wasm and motoko.wasm
    wasm_input("rust.wasm", false)
        .arg("check-endpoints")
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
    const HIDDEN_2: &str = r#"
    canister_update:dec
    "#;
    wasm_input("rust.wasm", false)
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_2).path())
        .assert()
        .stderr("ERROR: The following hidden endpoint is missing from the WASM exports section: canister_update:dec\n\
        Error: Canister WASM and Candid interface do not match!\n")
        .failure();
    wasm_input("motoko.wasm", false)
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
    wasm_input("motoko.wasm", false)
        .arg("check-endpoints")
        .arg("--hidden")
        .arg(create_tempfile(HIDDEN_3).path())
        .assert()
        .stdout("Canister WASM and Candid interface match!\n")
        .success();
}

fn create_tempfile(content: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
    write!(temp_file, "{content}").expect("Failed to write temp file content");
    temp_file
}

#[test]
fn stub_wasi() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let out_path = path.join("out-wasi.wasm");

    // First verify input has WASI imports
    let input_module = walrus::Module::from_file(path.join("wasi-test.wasm")).unwrap();
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
    wasm_input("wasi-test.wasm", false)
        .arg("-o")
        .arg(&out_path)
        .arg("instrument")
        .arg("--stub-wasi")
        .assert()
        .success();

    // Verify the output WASM has no WASI imports
    let module = walrus::Module::from_file(&out_path).unwrap();
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

    // Clean up
    fs::remove_file(&out_path).ok();
}
