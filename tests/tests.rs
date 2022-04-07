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
    wasm_input("greet.wasm", true)
        .arg("instrument")
        .assert()
        .success();
    assert_wasm("greet_instrument.wasm");
}

#[test]
fn metadata() {
    // List metadata
    wasm_input("greet.wasm", false)
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
    wasm_input("greet.wasm", false)
        .arg("metadata")
        .arg("motoko:compiler")
        .assert()
        .stdout("0.6.25\n")
        .success();
    // Get a non-existed metadata
    wasm_input("greet.wasm", false)
        .arg("metadata")
        .arg("whatever")
        .assert()
        .stdout("Cannot find metadata whatever\n")
        .success();
    // Overwrite motoko:compiler
    wasm_input("greet.wasm", true)
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
    wasm_input("greet.wasm", true)
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
