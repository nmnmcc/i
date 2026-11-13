// spec: 4.4, 4.7
// expect: compile-error E0504

// Choosing either generic variable for the Int bound loses a valid typing.
// The solver must report that it cannot find a principal type (§4.7),
// independently of which operand is written first.
choose = <A, B>(x: B & A, a: A, b: B) { (x + 1, a, b) }
use = () { choose(1, "s", 1) }
