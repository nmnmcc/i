// typed macro: T3
(m = spec/macros)
f = () { m.m(1) }
main = () { assert(f() == 7) }
