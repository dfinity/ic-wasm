actor {
  let evm : actor { request: shared () -> async (); requestCost: shared () -> async () } = actor "7hfb6-caaaa-aaaar-qadga-cai";
  public func requestCost() : async () {
    await evm.requestCost();
  };
  public func request() : async () {
    await evm.request();
  };
}

