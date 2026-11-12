// spec: 10.2, 2.2
// expect: compile-error E0301
// test-macro-migrate: reject

// Test attributes have no built-in meaning, including on the default entry.
@test
main = () { Nil }
