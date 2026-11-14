// spec: 7.1
// expect: compile-error E0110
// The old migration probe still fails for its specified key-order error.
g = ((a = b, c)) { c }
main = () { Nil }
