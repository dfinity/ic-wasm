#!ic-repl
// Check if the transformed Wasm module can be deployed

function install(wasm) {
  let id = call ic.provisional_create_canister_with_cycles(record { settings = null; amount = null });
  let S = id.canister_id;
  call ic.install_code(
    record {
      arg = encode ();
      wasm_module = wasm;
      mode = variant { install };
      canister_id = S;
    }
  );
  S
};

function motoko(wasm) {
  let S = install(wasm);
  call S.set(42);
  call S.inc();
  call S.get();
  assert _ == (43 : nat);
  S
};
function rust(wasm) {
  let S = install(wasm);
  call S.write((42 : nat));
  call S.inc();
  call S.read();
  assert _ == (43 : nat);
  S
};
function wat(wasm) {
  let S = install(wasm);
  call S.set((42 : int64));
  call S.inc();
  call S.get();
  assert _ == (43 : int64);
  S
};
function classes(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  call S.put(42, "text");
  call S.get(42);
  assert _ == opt "text";
  S
};
function classes_limit(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  fail call S.put(42, "text");
  assert _ ~= "0 cycles were received";
  S
};
function classes_redirect(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  fail call S.put(42, "text");
  assert _ ~= "No route to canister";
  S
};

let S = motoko(file("ok/motoko-instrument.wasm"));
call S.__get_cycles();
assert _ == (7199 : int64);
let S = motoko(file("ok/motoko-gc-instrument.wasm"));
call S.__get_cycles();
assert _ == (177 : int64);
motoko(file("ok/motoko-shrink.wasm"));
motoko(file("ok/motoko-limit.wasm"));

let S = rust(file("ok/rust-instrument.wasm"));
call S.__get_cycles();
assert _ == (66016 : int64);
rust(file("ok/rust-shrink.wasm"));
rust(file("ok/rust-limit.wasm"));

let S = wat(file("ok/wat-instrument.wasm"));
call S.__get_cycles();
assert _ == (121 : int64);
wat(file("ok/wat-shrink.wasm"));
wat(file("ok/wat-limit.wasm"));

classes(file("ok/classes-shrink.wasm"));
classes_limit(file("ok/classes-limit.wasm"));
classes_redirect(file("ok/classes-redirect.wasm"));
classes(file("ok/classes-nop-redirect.wasm"));
