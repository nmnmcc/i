export (same, A, B)
A {}
B {}
equal = (a, b) { a == b }
same = (a: A, b: B) { equal(a, b) }
