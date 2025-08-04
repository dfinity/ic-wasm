use assert_cmd::Command;

use std::fs;
use std::path::Path;

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
    wasm_input("wat.wasm", false)
        .arg("check-endpoints")
        .arg("--candid")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/valid.did"))
        .assert()
        .stdout("Canister WASM and Candid interface match!")
        .success();
    wasm_input("wat.wasm.gz", false)
        .arg("check-endpoints")
        .arg("--candid")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/valid.did"))
        .assert()
        .stdout("Canister WASM and Candid interface match!")
        .success();
}