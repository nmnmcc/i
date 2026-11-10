// spec: 9.7, 10.3
// expect: compile-error E0320

@host("test:io/v1")
Io { Io(get: () -> "a" | "b") }
