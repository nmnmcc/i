(m = probe/macros)
main = () { assert(m.id(42) == 42) assert(m.id(20, 22) == 42) }
