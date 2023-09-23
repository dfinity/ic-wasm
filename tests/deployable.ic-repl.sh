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
function upgrade(S, wasm) {
  call ic.install_code(
    record {
      arg = encode ();
      wasm_module = wasm;
      mode = variant { upgrade };
      canister_id = S;
    }
  );
};

function motoko(wasm) {
  let S = install(wasm);
  call S.set(42);
  call S.inc();
  call S.get();
  assert _ == (43 : nat);

  call S.inc();
  call S.inc();
  call S.get();
  assert _ == (45 : nat);
  S
};
function rust(wasm) {
  let S = install(wasm);
  call S.write((42 : nat));
  call S.inc();
  call S.read();
  assert _ == (43 : nat);

  call S.inc();
  call S.inc();
  call S.read();
  assert _ == (45 : nat);
  S
};
function wat(wasm) {
  let S = install(wasm);
  call S.set((42 : int64));
  call S.inc();
  call S.get();
  assert _ == (43 : int64);

  call S.inc();
  call S.inc();
  call S.get();
  assert _ == (45 : int64);
  S
};
function classes(wasm) {
  let S = install(wasm);
  call S.get(42);
  assert _ == (null : opt empty);
  call S.put(42, "text");
  call S.get(42);
  assert _ == opt "text";

  call S.put(40, "text0");
  call S.put(41, "text1");
  call S.put(42, "text2");
  call S.get(42);
  assert _ == opt "text2";
  metadata(S, "metadata/candid:args");
  assert _ == blob "()";
  metadata(S, "metadata/motoko:compiler");
  assert _ == blob "0.6.26";
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
  assert _ ~= "zz73r-nyaaa-aabbb-aaaca-cai not found";
  S
};
function check_profiling(S, cycles, len) {
  call S.__get_cycles();
  assert _ == (cycles : int64);
  call S.__get_profiling();
  assert _.size() == (len : nat);
  null
};

let S = motoko(file("ok/motoko-instrument.wasm"));
check_profiling(S, 9397, 78);
let S = motoko(file("ok/motoko-gc-instrument.wasm"));
check_profiling(S, 250, 4);
let wasm = file("ok/motoko-region-instrument.wasm");
let S = motoko(wasm);
check_profiling(S, 463589, 78);
upgrade(S, wasm);
call S.get();
assert _ == (45 : nat);
check_profiling(S, 472223, 374);
motoko(file("ok/motoko-shrink.wasm"));
motoko(file("ok/motoko-limit.wasm"));

let S = rust(file("ok/rust-instrument.wasm"));
check_profiling(S, 136378, 1654);
rust(file("ok/rust-shrink.wasm"));
rust(file("ok/rust-limit.wasm"));

let S = wat(file("ok/wat-instrument.wasm"));
check_profiling(S, 189, 2);
wat(file("ok/wat-shrink.wasm"));
wat(file("ok/wat-limit.wasm"));

classes(file("ok/classes-shrink.wasm"));
classes(file("ok/classes-optimize.wasm"));
classes(file("ok/classes-optimize-names.wasm"));
classes_limit(file("ok/classes-limit.wasm"));
classes_redirect(file("ok/classes-redirect.wasm"));
classes(file("ok/classes-nop-redirect.wasm"));
