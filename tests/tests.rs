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
        panic!(
            "ic_wasm did not result in expected wasm file: {} != {}",
            expected.display(),
            out.display()
        );
    }
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
    wasm_input("wat.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("wat-instrument.wasm");
    wasm_input("rust.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("rust-instrument.wasm");
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
    wasm_input("rust.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("rust-shrink.wasm");
    wasm_input("classes.wasm", true)
        .arg("shrink")
        .assert()
        .success();
    assert_wasm("classes-shrink.wasm")
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
}

#[test]
fn metadata() {
    // List metadata
    wasm_input("motoko.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(
            r#"icp:public candid:service
icp:private candid:args
icp:private motoko:stable-types
icp:private motoko:compiler
"#,
        )
        .success();
    // Get motoko:compiler content
    wasm_input("motoko.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.25\n")
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
icp:private candid:args
icp:private motoko:stable-types
icp:private motoko:compiler
icp:public whatever
"#,
        )
        .success();
}

#[test]
fn optimize() {
    let expected_metadata = r#"icp:public candid:service
icp:private candid:args
icp:private motoko:stable-types
icp:private motoko:compiler
"#;

    wasm_input("motoko.wasm", true)
        .arg("optimize")
        .assert()
        .success();
    assert_wasm("motoko-optimize.wasm");
    wasm_input("ok/motoko-optimize.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/motoko-optimize.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.25\n")
        .success();
    wasm_input("ok/motoko-optimize.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("motoko.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("O4")
        .assert()
        .success();
    assert_wasm("motoko-optimize-level-4.wasm");
    wasm_input("ok/motoko-optimize-level-4.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/motoko-optimize-level-4.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.25\n")
        .success();
    wasm_input("ok/motoko-optimize-level-4.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("motoko.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("Oz")
        .assert()
        .success();
    assert_wasm("motoko-optimize-level-z.wasm");
    wasm_input("ok/motoko-optimize-level-z.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/motoko-optimize-level-z.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.25\n")
        .success();
    wasm_input("ok/motoko-optimize-level-z.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("rust.wasm", true)
        .arg("optimize")
        .assert()
        .success();
    assert_wasm("rust-optimize.wasm");

    wasm_input("rust.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("O4")
        .assert()
        .success();
    assert_wasm("rust-optimize-level-4.wasm");

    wasm_input("rust.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("Oz")
        .assert()
        .success();
    assert_wasm("rust-optimize-level-z.wasm");

    wasm_input("classes.wasm", true)
        .arg("optimize")
        .assert()
        .success();
    assert_wasm("classes-optimize.wasm");
    wasm_input("ok/classes-optimize.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/classes-optimize.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.26\n")
        .success();
    wasm_input("ok/classes-optimize.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("classes.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("O4")
        .assert()
        .success();
    assert_wasm("classes-optimize-level-4.wasm");
    wasm_input("ok/classes-optimize-level-4.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/classes-optimize-level-4.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.26\n")
        .success();
    wasm_input("ok/classes-optimize-level-4.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("classes.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("Oz")
        .assert()
        .success();
    assert_wasm("classes-optimize-level-z.wasm");
    wasm_input("ok/classes-optimize-level-z.wasm", false)
        .arg("metadata")
        .assert()
        .stdout(expected_metadata)
        .success();
    wasm_input("ok/classes-optimize-level-z.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.26\n")
        .success();
    wasm_input("ok/classes-optimize-level-z.wasm", false)
        .arg("metadata")
        .arg("candid:args")
        .assert()
        .stdout("()\n")
        .success();

    wasm_input("wat.wasm", true)
        .arg("optimize")
        .assert()
        .success();
    assert_wasm("wat-optimize.wasm");

    wasm_input("wat.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("O4")
        .assert()
        .success();
    assert_wasm("wat-optimize-level-4.wasm");

    wasm_input("wat.wasm", true)
        .arg("optimize")
        .arg("--level")
        .arg("Oz")
        .assert()
        .success();
    assert_wasm("wat-optimize-level-z.wasm");
}
