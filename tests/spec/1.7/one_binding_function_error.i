// spec: 1.7
// expect: compile-error E0101
// old-syntax-check: E0101
(main)
f = (x, y) { x + y }
main = () { a = f  (b, c) = (1, 2) a }
