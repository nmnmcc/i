// spec: 7.1
// expect: compile-error E0510
// Reaching the nested pattern does not waive the other constructor keys.
P = (x = (Int, Int), y = Int)
k = (P(x = (a, b))) { a + b }
main = () { () }
