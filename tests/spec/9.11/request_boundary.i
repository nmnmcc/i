// spec: 10.3, 9.11
// expect: compile-error E0318
// runtime.md 5.2: Request itself never crosses an ordinary external boundary.
@external("test:request", "wrong")
wrong : () -> Request(Int)
