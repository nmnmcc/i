// spec: 10.1, 10.3
// expect: compile-error E0318

Level = "a" | "b"
@external("test:io/v1", "get")
get: () -> Level
