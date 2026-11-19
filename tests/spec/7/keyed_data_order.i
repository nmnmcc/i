// spec: 7.1
// expect: compile-error E0110
// old-syntax-check: E0110,E0101
// The old migration probe still fails for its specified key-order error.
g = ((a = b, c)) { c }
main = () { () }
