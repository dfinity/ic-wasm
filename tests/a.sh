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
  //call S.inc();
  //call S.get();
  //assert _ == (43 : nat);
  S
};
function rust(wasm) {
  let S = install(wasm);
  //call S.write((42 : nat));
  call S.inc();
  //call S.read();
  //assert _ == (43 : nat);
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

wat(wasm_profiling("wat.wasm"));
//let S = rust(wasm_profiling(file "rust.wasm"));

