// spec: 9.7, 10.3
// expect: compile-error E0320
// runtime.md 5.2: the Request payload must have an ABI result shape.
@host("test:request", "request")
Wrong { Wrong(start: () -> Request(List(Int))) }
