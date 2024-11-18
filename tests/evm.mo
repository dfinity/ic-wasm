actor {
  let evm : actor { request: shared () -> async (); requestCost: shared () -> async () } = actor "7hfb6-caaaa-aaaar-qadga-cai";
  let non_evm : actor { request: shared () -> async (); requestCost: shared () -> async () } = actor "cpmcr-yeaaa-aaaaa-qaala-cai";
  public func requestCost() : async () {
    await evm.requestCost();
  };
  public func request() : async () {
    await evm.request();
  };
  public func non_evm_request() : async () {
    await non_evm.request();
  };
}

