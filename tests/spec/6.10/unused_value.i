// spec: 6.10, 1.7
// expect: warning W0502

// `assert 1 + 1 == 2` is the two statements `assert` and `1 + 1 == 2`: the
// first does nothing, so the compiler warns about it.
@test
two_statements = () {
  assert 1 + 1 == 2
  Nil
}
