// The repeated parameter keeps its place (§4.3), so the number of
// parameters is 3 and `b` is reported where it is written.
C(A, A) = (A)

T(A, A, B) = C(A, A)
x: T(Int, Int, Int) = C(1)

y: T(Int, Int) = C(1)
