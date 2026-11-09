// spec: 9.6
// expect: compile-error E0701
// `assert` needs the `Assert` service, which comes later.

@test
asserts = () {
  assert(1 == 1)
}
