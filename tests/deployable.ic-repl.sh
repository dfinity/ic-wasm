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

let S = install(file "ok/motoko-instrument.wasm");
call S.set(42);
call S.inc();
call S.get();
assert _ == (43 : nat);

let S = install(file "ok/rust-instrument.wasm");
call S.write((42 : nat));
call S.inc();
call S.read();
assert _ == (43 : nat);

let S = install(file "ok/wat-instrument.wasm");
call S.set((42 : int64));
call S.inc();
call S.get();
assert _ == (43 : int64);

