// spec: 3, 6.8
// expect: pass
(main)
value = () -> Int ? String { 1 }
main = () { assert(value() == 1)  Nil }
