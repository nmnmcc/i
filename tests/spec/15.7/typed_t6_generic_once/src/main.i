// typed macro: T6
(m = spec/macros)
f = (x) { m.m(x) }
main = () { assert(f(1) == 1) assert(f("a") == "a") }
