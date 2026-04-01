(module
  ;; WASI imports
  (import "wasi_snapshot_preview1" "fd_close" (func $fd_close (param i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_write" (func $fd_write (param i32 i32 i32 i32) (result i32)))
  (import "wasi_snapshot_preview1" "fd_seek" (func $fd_seek (param i32 i64 i32 i32) (result i32)))
  
  ;; IC imports
  (import "ic0" "msg_reply" (func $msg_reply))
  (import "ic0" "msg_reply_data_append" (func $msg_reply_data_append (param i32 i32)))
  
  (memory 1)
  
  (func $test (export "canister_query test")
    ;; Call WASI functions (they will fail at runtime without stubs)
    i32.const 1
    call $fd_close
    drop
    
    call $msg_reply
  )
)
