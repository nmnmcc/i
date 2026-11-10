// old-syntax-check: E0105
outer(A) = (x: A) { inner(A) = (y: A) { y } (x, inner(42), inner("s")) }
