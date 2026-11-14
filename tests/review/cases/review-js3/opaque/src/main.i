export(same, A, B)
A = Opaque
B = Opaque
equal = (a, b) { a == b }
same = (a : A, b : B) { equal(a, b) }
