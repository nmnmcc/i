// spec: 4.9, 10.1, 10.3
// expect: compile-error E0318
@external("test:records/v1", "echo")
echo: (value: (name: String)) -> (name: String)
