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
};
function rust(wasm) {
  let S = install(wasm);
  call S.write((42 : nat));
  call S.inc();
  call S.read();
  assert _ == (43 : nat);  
};
function wat(wasm) {
  let S = install(wasm);
  call S.set((42 : int64));
  call S.inc();
  call S.get();
  assert _ == (43 : int64);  
};
function classes(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  call S.put(42, "text");
  call S.get(42);
  assert _ == opt "text";
};
function classes_limit(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  fail call S.put(42, "text");
  assert _ ~= "0 cycles were received";
};

motoko(file "ok/motoko-instrument.wasm");
motoko(file "ok/motoko-shrink.wasm");
motoko(file "ok/motoko-limit.wasm");
rust(file "ok/rust-instrument.wasm");
rust(file "ok/rust-shrink.wasm");
rust(file "ok/rust-limit.wasm");
wat(file "ok/wat-instrument.wasm");
wat(file "ok/wat-shrink.wasm");
wat(file "ok/wat-limit.wasm");
classes(file "ok/classes-shrink.wasm");
classes_limit(file "ok/classes-limit.wasm");


