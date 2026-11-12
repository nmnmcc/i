// spec: 3, 4.3, 4.7
// expect: compile-error E0532
Pair(A) = { (A, A) }

value: (Int, Int) = (1, 2)

Pair(Item) = value
