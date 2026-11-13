// old-syntax-check: E0101
outer(A) = (x: A) { inner(A) = (y: A) { y } (x, inner(42), inner("s")) }
