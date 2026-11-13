// spec: 3, 7.1
// expect: compile-error E0220
// Both alternatives are parsed, but every alternative must bind the same names.
y = 1
f = (x : Int | y) { 1 }
main = () { f(1) Nil }
