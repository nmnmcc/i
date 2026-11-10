// spec: 10.1, 10.3
// expect: pass

// §5.8: a finite scalar alias in an outgoing argument keeps the ordinary ABI.
Level = 1 | 2
@external("test:math/v1", "add")
add: (a: Level, b: Int) -> Int

@main
main = () { assert(add(1, 4) == 5) }
